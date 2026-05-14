use clap::{Args, ValueEnum};
use destack_workspace::{
    HeapOptionsJson, HeapSpaceOptionsJson, RandomModeJson, RandomOptionsJson, RuntimeOptionsJson,
    SchedulerModeJson, SchedulerOptionsJson, SchedulerPolicyJson, TimeModeJson, TimeOptionsJson,
    TraceModeJson, TraceOptionsJson,
};
use std::path::PathBuf;

/// Runtime configuration arguments for run-like commands.
#[derive(Args, Debug, Clone, Default)]
pub struct RuntimeArgs {
    /// Runtime execution mode.
    #[arg(long = "runtime-execution-mode", value_enum)]
    pub execution_mode: Option<ExecutionModeArg>,

    /// Replay path (file or directory).
    #[arg(long = "runtime-replay-path")]
    pub replay_path: Option<PathBuf>,

    /// Replay log filename template.
    #[arg(long = "runtime-replay-template")]
    pub replay_template: Option<String>,

    /// Replay log chunk size in megabytes.
    #[arg(long = "runtime-replay-chunk-mb")]
    pub replay_chunk_mb: Option<u64>,

    /// Runtime time mode.
    #[arg(long = "runtime-time-mode", value_enum)]
    pub time_mode: Option<TimeModeArg>,

    /// Runtime virtual time epoch in nanoseconds.
    #[arg(long = "runtime-time-epoch-ns")]
    pub time_epoch_ns: Option<u64>,

    /// Runtime time zone identifier.
    #[arg(long = "runtime-time-zone")]
    pub time_zone: Option<String>,

    /// Runtime randomness mode.
    #[arg(long = "runtime-random-mode", value_enum)]
    pub random_mode: Option<RandomModeArg>,

    /// Runtime randomness seed.
    #[arg(long = "runtime-random-seed")]
    pub random_seed: Option<u64>,

    /// Use a per-runnable random stream.
    #[arg(long = "runtime-random-per-runnable")]
    pub random_per_runnable: bool,

    /// Runtime scheduler policy.
    #[arg(long = "runtime-scheduler-policy", value_enum)]
    pub scheduler_policy: Option<SchedulerPolicyArg>,

    /// Runtime event loop tick budget in nanoseconds.
    #[arg(long = "runtime-scheduler-tick-budget-ns")]
    pub scheduler_tick_budget_ns: Option<u64>,

    /// Runtime microtask budget per tick.
    #[arg(long = "runtime-scheduler-microtask-budget")]
    pub scheduler_microtask_budget: Option<u64>,

    /// Runtime host semantic event budget before forcing one poller event.
    #[arg(long = "runtime-scheduler-host-event-budget")]
    pub scheduler_host_event_budget: Option<u64>,

    /// Runtime microtask nesting depth cap.
    #[arg(long = "runtime-scheduler-max-microtask-depth")]
    pub scheduler_max_microtask_depth: Option<u64>,

    /// Runtime timer resolution in nanoseconds.
    #[arg(long = "runtime-scheduler-timer-resolution-ns")]
    pub scheduler_timer_resolution_ns: Option<u64>,

    /// Runtime timer coalescing window in nanoseconds.
    #[arg(long = "runtime-scheduler-max-timer-coalesce-ns")]
    pub scheduler_max_timer_coalesce_ns: Option<u64>,

    /// Runtime preemption interval in nanoseconds.
    #[arg(long = "runtime-scheduler-preempt-interval-ns")]
    pub scheduler_preempt_interval_ns: Option<u64>,

    /// Runtime scheduler task cap.
    #[arg(long = "runtime-scheduler-task-limit")]
    pub scheduler_task_limit: Option<u64>,

    /// Runtime local-heap growth target percentage.
    #[arg(long = "runtime-heap-local-growth-percent")]
    pub heap_local_growth_percent: Option<u32>,

    /// Runtime local-heap soft memory limit in bytes.
    #[arg(long = "runtime-heap-local-memory-limit-bytes")]
    pub heap_local_memory_limit_bytes: Option<u64>,

    /// Runtime shared-heap growth target percentage.
    #[arg(long = "runtime-heap-shared-growth-percent")]
    pub heap_shared_growth_percent: Option<u32>,

    /// Runtime shared-heap soft memory limit in bytes.
    #[arg(long = "runtime-heap-shared-memory-limit-bytes")]
    pub heap_shared_memory_limit_bytes: Option<u64>,

    /// Runtime local-heap hard memory limit in bytes.
    #[arg(long = "runtime-heap-local-hard-limit-bytes")]
    pub heap_local_hard_limit_bytes: Option<u64>,

    /// Runtime shared-heap hard memory limit in bytes.
    #[arg(long = "runtime-heap-shared-hard-limit-bytes")]
    pub heap_shared_hard_limit_bytes: Option<u64>,
}

impl RuntimeArgs {
    /// Return true when any runtime override is set.
    pub fn is_empty(&self) -> bool {
        self.execution_mode.is_none()
            && self.replay_path.is_none()
            && self.replay_template.is_none()
            && self.replay_chunk_mb.is_none()
            && self.time_mode.is_none()
            && self.time_epoch_ns.is_none()
            && self.time_zone.is_none()
            && self.random_mode.is_none()
            && self.random_seed.is_none()
            && !self.random_per_runnable
            && self.scheduler_policy.is_none()
            && self.scheduler_tick_budget_ns.is_none()
            && self.scheduler_microtask_budget.is_none()
            && self.scheduler_host_event_budget.is_none()
            && self.scheduler_max_microtask_depth.is_none()
            && self.scheduler_timer_resolution_ns.is_none()
            && self.scheduler_max_timer_coalesce_ns.is_none()
            && self.scheduler_preempt_interval_ns.is_none()
            && self.scheduler_task_limit.is_none()
            && self.heap_local_growth_percent.is_none()
            && self.heap_local_memory_limit_bytes.is_none()
            && self.heap_shared_growth_percent.is_none()
            && self.heap_shared_memory_limit_bytes.is_none()
            && self.heap_local_hard_limit_bytes.is_none()
            && self.heap_shared_hard_limit_bytes.is_none()
    }

    /// Convert runtime arguments into runtime option overrides.
    pub fn to_runtime_overrides(&self) -> Option<RuntimeOptionsJson> {
        if self.is_empty() {
            return None;
        }

        let trace = if self.execution_mode.is_some()
            || self.replay_path.is_some()
            || self.replay_template.is_some()
            || self.replay_chunk_mb.is_some()
        {
            Some(TraceOptionsJson {
                mode: self.execution_mode.map(TraceModeJson::from),
                restore: None,
                path: self
                    .replay_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into()),
                template: self.replay_template.clone(),
                chunk_size_mb: self.replay_chunk_mb,
                payload: None,
            })
        } else {
            None
        };

        let time =
            if self.time_mode.is_some() || self.time_epoch_ns.is_some() || self.time_zone.is_some()
            {
                Some(TimeOptionsJson {
                    mode: self.time_mode.map(Into::into),
                    epoch_ns: self.time_epoch_ns,
                    time_zone: self.time_zone.clone(),
                })
            } else {
                None
            };

        let random =
            if self.random_mode.is_some() || self.random_seed.is_some() || self.random_per_runnable
            {
                Some(RandomOptionsJson {
                    mode: self.random_mode.map(Into::into),
                    seed: self.random_seed,
                    per_runnable: self.random_per_runnable.then_some(true),
                })
            } else {
                None
            };

        let scheduler = if self.scheduler_policy.is_some()
            || self.scheduler_tick_budget_ns.is_some()
            || self.scheduler_microtask_budget.is_some()
            || self.scheduler_host_event_budget.is_some()
            || self.scheduler_max_microtask_depth.is_some()
            || self.scheduler_timer_resolution_ns.is_some()
            || self.scheduler_max_timer_coalesce_ns.is_some()
            || self.scheduler_preempt_interval_ns.is_some()
            || self.scheduler_task_limit.is_some()
            || self.execution_mode.is_some()
        {
            Some(SchedulerOptionsJson {
                mode: self.execution_mode.map(SchedulerModeJson::from),
                tick_budget_ns: self.scheduler_tick_budget_ns,
                microtask_budget: self.scheduler_microtask_budget,
                host_event_budget: self.scheduler_host_event_budget,
                max_microtask_depth: self.scheduler_max_microtask_depth,
                timer_resolution_ns: self.scheduler_timer_resolution_ns,
                max_timer_coalesce_ns: self.scheduler_max_timer_coalesce_ns,
                preempt_interval_ns: self.scheduler_preempt_interval_ns,
                policy: self.scheduler_policy.map(Into::into),
                task_limit: self.scheduler_task_limit,
                poller_backend: None,
            })
        } else {
            None
        };

        let heap = if self.heap_local_growth_percent.is_some()
            || self.heap_local_memory_limit_bytes.is_some()
            || self.heap_local_hard_limit_bytes.is_some()
            || self.heap_shared_growth_percent.is_some()
            || self.heap_shared_memory_limit_bytes.is_some()
            || self.heap_shared_hard_limit_bytes.is_some()
        {
            Some(HeapOptionsJson {
                local: Some(HeapSpaceOptionsJson {
                    growth_percent: self.heap_local_growth_percent,
                    memory_limit_bytes: self.heap_local_memory_limit_bytes,
                    hard_limit_bytes: self.heap_local_hard_limit_bytes,
                }),
                shared: Some(HeapSpaceOptionsJson {
                    growth_percent: self.heap_shared_growth_percent,
                    memory_limit_bytes: self.heap_shared_memory_limit_bytes,
                    hard_limit_bytes: self.heap_shared_hard_limit_bytes,
                }),
                allocator: None,
            })
        } else {
            None
        };

        Some(RuntimeOptionsJson {
            scheduler,
            trace,
            time,
            random,
            heap,
            platform: None,
            ..Default::default()
        })
    }
}

/// Execution mode for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExecutionModeArg {
    /// Fast execution without determinism guarantees.
    Fast,
    /// Deterministic scheduling with controlled randomness.
    Deterministic,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

impl From<ExecutionModeArg> for SchedulerModeJson {
    fn from(value: ExecutionModeArg) -> Self {
        match value {
            ExecutionModeArg::Fast => Self::Parallel,
            ExecutionModeArg::Deterministic
            | ExecutionModeArg::Record
            | ExecutionModeArg::Replay => Self::Cooperative,
        }
    }
}

impl From<ExecutionModeArg> for TraceModeJson {
    fn from(value: ExecutionModeArg) -> Self {
        match value {
            ExecutionModeArg::Fast | ExecutionModeArg::Deterministic => Self::Off,
            ExecutionModeArg::Record => Self::Record,
            ExecutionModeArg::Replay => Self::Replay,
        }
    }
}

/// Scheduler policy for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SchedulerPolicyArg {
    /// First-in, first-out scheduling.
    Fifo,
    /// Fair scheduling with time slicing.
    Fair,
    /// Work-stealing scheduling for throughput.
    #[value(alias = "work_stealing")]
    WorkStealing,
}

impl From<SchedulerPolicyArg> for SchedulerPolicyJson {
    fn from(value: SchedulerPolicyArg) -> Self {
        match value {
            SchedulerPolicyArg::Fifo => SchedulerPolicyJson::Fifo,
            SchedulerPolicyArg::Fair => SchedulerPolicyJson::Fair,
            SchedulerPolicyArg::WorkStealing => SchedulerPolicyJson::WorkStealing,
        }
    }
}

/// Time mode for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TimeModeArg {
    /// Use the host clock directly.
    Host,
    /// Use a virtualized clock derived from runtime state.
    Virtual,
}

impl From<TimeModeArg> for TimeModeJson {
    fn from(value: TimeModeArg) -> Self {
        match value {
            TimeModeArg::Host => Self::Host,
            TimeModeArg::Virtual => Self::Virtual,
        }
    }
}

/// Randomness mode for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RandomModeArg {
    /// Use the host randomness source.
    Host,
    /// Use deterministic runtime-managed randomness.
    Deterministic,
}

impl From<RandomModeArg> for RandomModeJson {
    fn from(value: RandomModeArg) -> Self {
        match value {
            RandomModeArg::Host => Self::Host,
            RandomModeArg::Deterministic => Self::Deterministic,
        }
    }
}
