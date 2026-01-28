use std::path::PathBuf;

use clap::{Args, ValueEnum};
use destack_workspace::{
    DeterminismPolicyJson, DsConfigRuntimeOptionsJson, GcLoggingJson, GcOptionsJson,
    RandomModeJson, RandomOptionsJson, ReplayLogOptionsJson, ReplayModeJson, SchedulerOptionsJson,
    SchedulerPolicyJson, TimeModeJson, TimeOptionsJson,
};

/// Runtime configuration arguments for run-like commands.
#[derive(Args, Debug, Clone, Default)]
pub struct RuntimeArgs {
    /// Runtime determinism policy.
    #[arg(long = "runtime-determinism", value_enum)]
    pub determinism: Option<DeterminismArg>,

    /// Runtime replay mode.
    #[arg(long = "runtime-replay", value_enum)]
    pub replay: Option<ReplayArg>,

    /// Replay log path (file or directory).
    #[arg(long = "runtime-replay-log")]
    pub replay_log_path: Option<PathBuf>,

    /// Replay log filename template.
    #[arg(long = "runtime-replay-template")]
    pub replay_log_template: Option<String>,

    /// Replay log chunk size in megabytes.
    #[arg(long = "runtime-replay-chunk-mb")]
    pub replay_log_chunk_mb: Option<u64>,

    /// Runtime time mode.
    #[arg(long = "runtime-time-mode", value_enum)]
    pub time_mode: Option<TimeModeArg>,

    /// Runtime virtual time epoch in nanoseconds.
    #[arg(long = "runtime-time-epoch-ns")]
    pub time_epoch_ns: Option<u64>,

    /// Runtime virtual time tick size in nanoseconds.
    #[arg(long = "runtime-time-tick-ns")]
    pub time_tick_ns: Option<u64>,

    /// Runtime time zone identifier.
    #[arg(long = "runtime-time-zone")]
    pub time_zone: Option<String>,

    /// Runtime randomness mode.
    #[arg(long = "runtime-random-mode", value_enum)]
    pub random_mode: Option<RandomModeArg>,

    /// Runtime randomness seed.
    #[arg(long = "runtime-random-seed")]
    pub random_seed: Option<u64>,

    /// Use a per-task random stream.
    #[arg(long = "runtime-random-per-task")]
    pub random_per_task: bool,

    /// Runtime scheduler policy.
    #[arg(long = "runtime-scheduler-policy", value_enum)]
    pub scheduler_policy: Option<SchedulerPolicyArg>,

    /// Runtime event loop tick budget in nanoseconds.
    #[arg(long = "runtime-scheduler-tick-budget-ns")]
    pub scheduler_tick_budget_ns: Option<u64>,

    /// Runtime microtask budget per tick.
    #[arg(long = "runtime-scheduler-microtask-budget")]
    pub scheduler_microtask_budget: Option<u64>,

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

    /// Enable or disable the garbage collector.
    #[arg(long = "runtime-gc-enabled")]
    pub gc_enabled: Option<bool>,

    /// Runtime GC heap growth target percentage.
    #[arg(long = "runtime-gc-heap-growth-percent")]
    pub gc_heap_growth_percent: Option<u32>,

    /// Runtime GC soft heap limit in bytes.
    #[arg(long = "runtime-gc-heap-soft-limit-bytes")]
    pub gc_heap_soft_limit_bytes: Option<u64>,

    /// Runtime GC initial heap size hint in bytes.
    #[arg(long = "runtime-gc-heap-initial-bytes")]
    pub gc_heap_initial_bytes: Option<u64>,

    /// Runtime GC logging verbosity.
    #[arg(long = "runtime-gc-logging", value_enum)]
    pub gc_logging: Option<GcLoggingArg>,
}

impl RuntimeArgs {
    /// Return true when any runtime override is set.
    pub fn is_empty(&self) -> bool {
        self.determinism.is_none()
            && self.replay.is_none()
            && self.replay_log_path.is_none()
            && self.replay_log_template.is_none()
            && self.replay_log_chunk_mb.is_none()
            && self.time_mode.is_none()
            && self.time_epoch_ns.is_none()
            && self.time_tick_ns.is_none()
            && self.time_zone.is_none()
            && self.random_mode.is_none()
            && self.random_seed.is_none()
            && !self.random_per_task
            && self.scheduler_policy.is_none()
            && self.scheduler_tick_budget_ns.is_none()
            && self.scheduler_microtask_budget.is_none()
            && self.scheduler_max_microtask_depth.is_none()
            && self.scheduler_timer_resolution_ns.is_none()
            && self.scheduler_max_timer_coalesce_ns.is_none()
            && self.scheduler_preempt_interval_ns.is_none()
            && self.scheduler_worker_threads.is_none()
            && self.scheduler_io_threads.is_none()
            && self.scheduler_blocking_threads.is_none()
            && self.scheduler_max_tasks.is_none()
            && self.gc_enabled.is_none()
            && self.gc_heap_growth_percent.is_none()
            && self.gc_heap_soft_limit_bytes.is_none()
            && self.gc_heap_initial_bytes.is_none()
            && self.gc_logging.is_none()
    }

    /// Convert runtime arguments into runtime option overrides.
    pub fn to_runtime_overrides(&self) -> Option<DsConfigRuntimeOptionsJson> {
        if self.is_empty() {
            return None;
        }

        let replay_log = if self.replay_log_path.is_some()
            || self.replay_log_template.is_some()
            || self.replay_log_chunk_mb.is_some()
        {
            Some(ReplayLogOptionsJson {
                path: self
                    .replay_log_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into()),
                template: self.replay_log_template.clone(),
                chunk_size_mb: self.replay_log_chunk_mb,
            })
        } else {
            None
        };

        let time = if self.time_mode.is_some()
            || self.time_epoch_ns.is_some()
            || self.time_tick_ns.is_some()
            || self.time_zone.is_some()
        {
            Some(TimeOptionsJson {
                mode: self.time_mode.map(Into::into),
                epoch_ns: self.time_epoch_ns,
                tick_ns: self.time_tick_ns,
                time_zone: self.time_zone.clone(),
            })
        } else {
            None
        };

        let random =
            if self.random_mode.is_some() || self.random_seed.is_some() || self.random_per_task {
                Some(RandomOptionsJson {
                    mode: self.random_mode.map(Into::into),
                    seed: self.random_seed,
                    per_task: if self.random_per_task {
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
                max_microtask_depth: self.scheduler_max_microtask_depth,
                timer_resolution_ns: self.scheduler_timer_resolution_ns,
                max_timer_coalesce_ns: self.scheduler_max_timer_coalesce_ns,
                preempt_interval_ns: self.scheduler_preempt_interval_ns,
                policy: self.scheduler_policy.map(Into::into),
                worker_threads: self.scheduler_worker_threads,
                io_threads: self.scheduler_io_threads,
                blocking_threads: self.scheduler_blocking_threads,
                max_tasks: self.scheduler_max_tasks,
            })
        } else {
            None
        };

        let gc = if self.gc_enabled.is_some()
            || self.gc_heap_growth_percent.is_some()
            || self.gc_heap_soft_limit_bytes.is_some()
            || self.gc_heap_initial_bytes.is_some()
            || self.gc_logging.is_some()
        {
            Some(GcOptionsJson {
                enabled: self.gc_enabled,
                heap_growth_percent: self.gc_heap_growth_percent,
                heap_soft_limit_bytes: self.gc_heap_soft_limit_bytes,
                heap_initial_bytes: self.gc_heap_initial_bytes,
                logging: self.gc_logging.map(Into::into),
            })
        } else {
            None
        };

        Some(DsConfigRuntimeOptionsJson {
            determinism: self.determinism.map(Into::into),
            replay: self.replay.map(Into::into),
            replay_log,
            time,
            random,
            scheduler,
            gc,
        })
    }
}

/// Determinism policy for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DeterminismArg {
    /// Best-effort execution without determinism guarantees.
    BestEffort,
    /// Deterministic scheduling with controlled randomness.
    Deterministic,
}

impl From<DeterminismArg> for DeterminismPolicyJson {
    fn from(value: DeterminismArg) -> Self {
        match value {
            DeterminismArg::BestEffort => DeterminismPolicyJson::BestEffort,
            DeterminismArg::Deterministic => DeterminismPolicyJson::Deterministic,
        }
    }
}

/// Replay policy for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReplayArg {
    /// Disable record/replay.
    Off,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

impl From<ReplayArg> for ReplayModeJson {
    fn from(value: ReplayArg) -> Self {
        match value {
            ReplayArg::Off => ReplayModeJson::Off,
            ReplayArg::Record => ReplayModeJson::Record,
            ReplayArg::Replay => ReplayModeJson::Replay,
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

/// GC logging for CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GcLoggingArg {
    /// Disable GC logging.
    Off,
    /// Emit summary GC events.
    Summary,
    /// Emit verbose GC events.
    Verbose,
}

impl From<GcLoggingArg> for GcLoggingJson {
    fn from(value: GcLoggingArg) -> Self {
        match value {
            GcLoggingArg::Off => GcLoggingJson::Off,
            GcLoggingArg::Summary => GcLoggingJson::Summary,
            GcLoggingArg::Verbose => GcLoggingJson::Verbose,
        }
    }
}
