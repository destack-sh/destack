use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, ValueEnum};
use destack_workspace::{
    ExecutionModeJson, HeapGcOptionsJson, HeapLayoutOptionsJson, HeapLimitOptionsJson,
    HeapOptionsJson, HeapSizeClassesJson, LocalGcOptionsJson, LocalHeapLimitOptionsJson,
    RandomModeJson, RandomOptionsJson, ReplayOptionsJson, RuntimeAccessJson, RuntimeOptionsJson,
    RuntimeWorldJson, SchedulerOptionsJson, SchedulerPolicyJson, SharedGcOptionsJson,
    SharedHeapLimitOptionsJson, TimeModeJson, TimeOptionsJson,
};

/// Runtime configuration arguments for run-like commands.
#[derive(Args, Debug, Clone, Default)]
pub struct RuntimeArgs {
    /// Runtime execution mode.
    #[arg(long = "runtime-execution-mode", value_enum)]
    pub execution_mode: Option<ExecutionModeArg>,

    /// Runtime default world for external bindings.
    #[arg(long = "runtime-world", value_enum)]
    pub world: Option<RuntimeWorldArg>,

    /// Runtime default access policy for external bindings.
    #[arg(long = "runtime-access", value_enum)]
    pub access: Option<RuntimeAccessArg>,

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

    /// Runtime worker thread count.
    #[arg(long = "runtime-scheduler-worker-threads")]
    pub scheduler_worker_threads: Option<u64>,

    /// Runtime I/O thread count.
    #[arg(long = "runtime-scheduler-io-threads")]
    pub scheduler_io_threads: Option<u64>,

    /// Runtime blocking thread count.
    #[arg(long = "runtime-scheduler-blocking-threads")]
    pub scheduler_blocking_threads: Option<u64>,

    /// Runtime scheduler task cap.
    #[arg(long = "runtime-scheduler-max-tasks")]
    pub scheduler_max_tasks: Option<u64>,

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

    /// Runtime heap size-class table preset or comma-separated byte list.
    #[arg(long = "runtime-heap-layout-size-classes")]
    pub heap_layout_size_classes: Option<HeapSizeClassesArg>,

    /// Managed young-space size in bytes.
    #[arg(long = "runtime-heap-layout-managed-young-bytes")]
    pub heap_layout_managed_young_bytes: Option<usize>,

    /// Maximum payload size admitted into managed young space.
    #[arg(long = "runtime-heap-layout-max-managed-young-allocation-bytes")]
    pub heap_layout_max_managed_young_allocation_bytes: Option<usize>,

    /// Managed small-allocation span size in bytes.
    #[arg(long = "runtime-heap-layout-managed-span-bytes")]
    pub heap_layout_managed_span_bytes: Option<usize>,

    /// Raw small-allocation span size in bytes.
    #[arg(long = "runtime-heap-layout-raw-span-bytes")]
    pub heap_layout_raw_span_bytes: Option<usize>,

    /// Heap page size in bytes.
    #[arg(long = "runtime-heap-layout-page-bytes")]
    pub heap_layout_page_bytes: Option<usize>,

    /// Heap segment size in bytes.
    #[arg(long = "runtime-heap-layout-segment-bytes")]
    pub heap_layout_segment_bytes: Option<usize>,

    /// Remembered-card width in bytes.
    #[arg(long = "runtime-heap-layout-card-bytes")]
    pub heap_layout_card_bytes: Option<usize>,

    /// Small-allocation alignment in bytes.
    #[arg(long = "runtime-heap-layout-small-alignment-bytes")]
    pub heap_layout_small_alignment_bytes: Option<usize>,

    /// Hard local-heap total retained-byte limit.
    #[arg(long = "runtime-heap-limit-local-max-bytes")]
    pub heap_limit_local_max_bytes: Option<u64>,

    /// Hard local managed-space retained-byte limit.
    #[arg(long = "runtime-heap-limit-local-managed-max-bytes")]
    pub heap_limit_local_managed_max_bytes: Option<u64>,

    /// Hard local raw-space retained-byte limit.
    #[arg(long = "runtime-heap-limit-local-raw-max-bytes")]
    pub heap_limit_local_raw_max_bytes: Option<u64>,

    /// Hard shared-heap total retained-byte limit.
    #[arg(long = "runtime-heap-limit-shared-max-bytes")]
    pub heap_limit_shared_max_bytes: Option<u64>,

    /// Hard shared managed-space retained-byte limit.
    #[arg(long = "runtime-heap-limit-shared-managed-max-bytes")]
    pub heap_limit_shared_managed_max_bytes: Option<u64>,

    /// Hard shared raw-space retained-byte limit.
    #[arg(long = "runtime-heap-limit-shared-raw-max-bytes")]
    pub heap_limit_shared_raw_max_bytes: Option<u64>,
}

impl RuntimeArgs {
    /// Return true when any runtime override is set.
    pub fn is_empty(&self) -> bool {
        self.execution_mode.is_none()
            && self.world.is_none()
            && self.access.is_none()
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
            && self.scheduler_worker_threads.is_none()
            && self.scheduler_io_threads.is_none()
            && self.scheduler_blocking_threads.is_none()
            && self.scheduler_max_tasks.is_none()
            && self.heap_local_growth_percent.is_none()
            && self.heap_local_memory_limit_bytes.is_none()
            && self.heap_shared_growth_percent.is_none()
            && self.heap_shared_memory_limit_bytes.is_none()
            && self.heap_layout_size_classes.is_none()
            && self.heap_layout_managed_young_bytes.is_none()
            && self
                .heap_layout_max_managed_young_allocation_bytes
                .is_none()
            && self.heap_layout_managed_span_bytes.is_none()
            && self.heap_layout_raw_span_bytes.is_none()
            && self.heap_layout_page_bytes.is_none()
            && self.heap_layout_segment_bytes.is_none()
            && self.heap_layout_card_bytes.is_none()
            && self.heap_layout_small_alignment_bytes.is_none()
            && self.heap_limit_local_max_bytes.is_none()
            && self.heap_limit_local_managed_max_bytes.is_none()
            && self.heap_limit_local_raw_max_bytes.is_none()
            && self.heap_limit_shared_max_bytes.is_none()
            && self.heap_limit_shared_managed_max_bytes.is_none()
            && self.heap_limit_shared_raw_max_bytes.is_none()
    }

    /// Convert runtime arguments into runtime option overrides.
    pub fn to_runtime_overrides(&self) -> Option<RuntimeOptionsJson> {
        if self.is_empty() {
            return None;
        }

        let replay = if self.replay_path.is_some()
            || self.replay_template.is_some()
            || self.replay_chunk_mb.is_some()
        {
            Some(ReplayOptionsJson {
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
                    per_runnable: if self.random_per_runnable {
                        Some(true)
                    } else {
                        None
                    },
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
            || self.scheduler_worker_threads.is_some()
            || self.scheduler_io_threads.is_some()
            || self.scheduler_blocking_threads.is_some()
            || self.scheduler_max_tasks.is_some()
        {
            Some(SchedulerOptionsJson {
                tick_budget_ns: self.scheduler_tick_budget_ns,
                microtask_budget: self.scheduler_microtask_budget,
                host_event_budget: self.scheduler_host_event_budget,
                max_microtask_depth: self.scheduler_max_microtask_depth,
                timer_resolution_ns: self.scheduler_timer_resolution_ns,
                max_timer_coalesce_ns: self.scheduler_max_timer_coalesce_ns,
                preempt_interval_ns: self.scheduler_preempt_interval_ns,
                policy: self.scheduler_policy.map(Into::into),
                worker_threads: self.scheduler_worker_threads,
                io_threads: self.scheduler_io_threads,
                blocking_threads: self.scheduler_blocking_threads,
                max_tasks: self.scheduler_max_tasks,
                poller_backend: None,
            })
        } else {
            None
        };

        let heap = if self.heap_local_growth_percent.is_some()
            || self.heap_local_memory_limit_bytes.is_some()
            || self.heap_shared_growth_percent.is_some()
            || self.heap_shared_memory_limit_bytes.is_some()
            || self.heap_layout_size_classes.is_some()
            || self.heap_layout_managed_young_bytes.is_some()
            || self
                .heap_layout_max_managed_young_allocation_bytes
                .is_some()
            || self.heap_layout_managed_span_bytes.is_some()
            || self.heap_layout_raw_span_bytes.is_some()
            || self.heap_layout_page_bytes.is_some()
            || self.heap_layout_segment_bytes.is_some()
            || self.heap_layout_card_bytes.is_some()
            || self.heap_layout_small_alignment_bytes.is_some()
            || self.heap_limit_local_max_bytes.is_some()
            || self.heap_limit_local_managed_max_bytes.is_some()
            || self.heap_limit_local_raw_max_bytes.is_some()
            || self.heap_limit_shared_max_bytes.is_some()
            || self.heap_limit_shared_managed_max_bytes.is_some()
            || self.heap_limit_shared_raw_max_bytes.is_some()
        {
            Some(HeapOptionsJson {
                gc: Some(HeapGcOptionsJson {
                    local: Some(LocalGcOptionsJson {
                        growth_percent: self.heap_local_growth_percent,
                        memory_limit_bytes: self.heap_local_memory_limit_bytes,
                    }),
                    shared: Some(SharedGcOptionsJson {
                        growth_percent: self.heap_shared_growth_percent,
                        memory_limit_bytes: self.heap_shared_memory_limit_bytes,
                    }),
                }),
                limit: Some(HeapLimitOptionsJson {
                    local: Some(LocalHeapLimitOptionsJson {
                        max_bytes: self.heap_limit_local_max_bytes,
                        managed_max_bytes: self.heap_limit_local_managed_max_bytes,
                        raw_max_bytes: self.heap_limit_local_raw_max_bytes,
                    }),
                    shared: Some(SharedHeapLimitOptionsJson {
                        max_bytes: self.heap_limit_shared_max_bytes,
                        managed_max_bytes: self.heap_limit_shared_managed_max_bytes,
                        raw_max_bytes: self.heap_limit_shared_raw_max_bytes,
                    }),
                }),
                layout: Some(HeapLayoutOptionsJson {
                    size_classes: self
                        .heap_layout_size_classes
                        .as_ref()
                        .map(HeapSizeClassesArg::to_json),
                    managed_young_bytes: self.heap_layout_managed_young_bytes,
                    max_managed_young_allocation_bytes: self
                        .heap_layout_max_managed_young_allocation_bytes,
                    managed_span_bytes: self.heap_layout_managed_span_bytes,
                    raw_span_bytes: self.heap_layout_raw_span_bytes,
                    page_bytes: self.heap_layout_page_bytes,
                    segment_bytes: self.heap_layout_segment_bytes,
                    card_bytes: self.heap_layout_card_bytes,
                    small_alignment_bytes: self.heap_layout_small_alignment_bytes,
                }),
            })
        } else {
            None
        };

        Some(RuntimeOptionsJson {
            execution: self.execution_mode.map(Into::into),
            world: self.world.map(Into::into),
            access: self.access.map(Into::into),
            replay,
            time,
            random,
            scheduler,
            heap,
            platform: None,
            ..Default::default()
        })
    }
}

/// Heap size-class argument for CLI flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapSizeClassesArg {
    /// One named built-in size-class table.
    Named(String),
    /// One explicit size-class table in bytes.
    Explicit(Vec<usize>),
}

impl HeapSizeClassesArg {
    /// Convert this CLI argument into workspace JSON.
    pub fn to_json(&self) -> HeapSizeClassesJson {
        match self {
            Self::Named(name) => HeapSizeClassesJson::Named(name.clone()),
            Self::Explicit(classes) => HeapSizeClassesJson::Explicit(classes.clone()),
        }
    }
}

impl FromStr for HeapSizeClassesArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err("heap size classes must not be empty".into());
        }

        // named preset
        if !value.contains(',') {
            return Ok(Self::Named(value.to_string()));
        }

        // explicit table
        let mut classes = Vec::new();
        for part in value.split(',') {
            let part = part.trim();
            if part.is_empty() {
                return Err("heap size-class lists must not contain empty entries".into());
            }

            let bytes = part
                .parse::<usize>()
                .map_err(|_| format!("invalid heap size-class byte width: {part}"))?;
            classes.push(bytes);
        }

        Ok(Self::Explicit(classes))
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

impl From<ExecutionModeArg> for ExecutionModeJson {
    fn from(value: ExecutionModeArg) -> Self {
        match value {
            ExecutionModeArg::Fast => ExecutionModeJson::Fast,
            ExecutionModeArg::Deterministic => ExecutionModeJson::Deterministic,
            ExecutionModeArg::Record => ExecutionModeJson::Record,
            ExecutionModeArg::Replay => ExecutionModeJson::Replay,
        }
    }
}

/// Runtime world for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RuntimeWorldArg {
    /// Use host-backed platform bindings.
    Host,
    /// Use simulation-backed platform bindings.
    Simulation,
}

impl From<RuntimeWorldArg> for RuntimeWorldJson {
    fn from(value: RuntimeWorldArg) -> Self {
        match value {
            RuntimeWorldArg::Host => RuntimeWorldJson::Host,
            RuntimeWorldArg::Simulation => RuntimeWorldJson::Simulation,
        }
    }
}

/// Runtime default access policy for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RuntimeAccessArg {
    /// Allow matching binding calls.
    Allow,
    /// Deny matching binding calls.
    Deny,
}

impl From<RuntimeAccessArg> for RuntimeAccessJson {
    fn from(value: RuntimeAccessArg) -> Self {
        match value {
            RuntimeAccessArg::Allow => RuntimeAccessJson::Allow,
            RuntimeAccessArg::Deny => RuntimeAccessJson::Deny,
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
            TimeModeArg::Host => TimeModeJson::Host,
            TimeModeArg::Virtual => TimeModeJson::Virtual,
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
            RandomModeArg::Host => RandomModeJson::Host,
            RandomModeArg::Deterministic => RandomModeJson::Deterministic,
        }
    }
}
