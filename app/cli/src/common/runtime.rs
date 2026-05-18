use clap::{Args, ValueEnum};
use destack_workspace::ConfigPatch;
use serde_json::{Map, Value};
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

    /// Convert runtime arguments into one config patch.
    pub fn to_config_patch(&self) -> Option<ConfigPatch> {
        if self.is_empty() {
            return None;
        }

        let mut runtime = Map::new();

        // apply collapsed execution mode
        if let Some(mode) = self.execution_mode {
            insert_runtime_value(&mut runtime, "scheduler", "mode", mode.scheduler_value());
            insert_runtime_value(&mut runtime, "trace", "mode", mode.trace_value());
            if mode == ExecutionModeArg::Replay {
                insert_runtime_value(
                    &mut runtime,
                    "time",
                    "source",
                    Value::String("virtual".into()),
                );
                insert_runtime_value(
                    &mut runtime,
                    "random",
                    "source",
                    Value::String("deterministic".into()),
                );
            }
        }

        // apply trace settings
        if let Some(path) = self.replay_path.as_ref() {
            insert_runtime_value(
                &mut runtime,
                "trace",
                "path",
                Value::String(path.to_string_lossy().into_owned()),
            );
        }
        if let Some(template) = self.replay_template.as_ref() {
            insert_runtime_value(
                &mut runtime,
                "trace",
                "template",
                Value::String(template.clone()),
            );
        }
        if let Some(chunk_size_mb) = self.replay_chunk_mb {
            insert_runtime_value(&mut runtime, "trace", "chunkSizeMb", chunk_size_mb.into());
        }

        // apply time settings
        if let Some(mode) = self.time_mode {
            insert_runtime_value(&mut runtime, "time", "source", mode.value());
        }
        if let Some(epoch_ns) = self.time_epoch_ns {
            insert_runtime_value(&mut runtime, "time", "epochNs", epoch_ns.into());
        }
        if let Some(time_zone) = self.time_zone.as_ref() {
            insert_runtime_value(
                &mut runtime,
                "time",
                "timeZone",
                Value::String(time_zone.clone()),
            );
        }

        // apply randomness settings
        if let Some(mode) = self.random_mode {
            insert_runtime_value(&mut runtime, "random", "source", mode.value());
        }
        if let Some(seed) = self.random_seed {
            insert_runtime_value(&mut runtime, "random", "seed", seed.into());
        }
        if self.random_per_runnable {
            insert_runtime_value(&mut runtime, "random", "perRunnable", true.into());
        }

        // apply scheduler settings
        if let Some(policy) = self.scheduler_policy {
            insert_runtime_value(&mut runtime, "scheduler", "policy", policy.value());
        }
        if let Some(tick_budget_ns) = self.scheduler_tick_budget_ns {
            insert_runtime_value(
                &mut runtime,
                "scheduler",
                "tickBudgetNs",
                tick_budget_ns.into(),
            );
        }
        if let Some(microtask_budget) = self.scheduler_microtask_budget {
            insert_runtime_value(
                &mut runtime,
                "scheduler",
                "microtaskBudget",
                microtask_budget.into(),
            );
        }
        if let Some(max_microtask_depth) = self.scheduler_max_microtask_depth {
            insert_runtime_value(
                &mut runtime,
                "scheduler",
                "maxMicrotaskDepth",
                max_microtask_depth.into(),
            );
        }
        if let Some(timer_resolution_ns) = self.scheduler_timer_resolution_ns {
            insert_runtime_value(
                &mut runtime,
                "scheduler",
                "timerResolutionNs",
                timer_resolution_ns.into(),
            );
        }
        if let Some(max_timer_coalesce_ns) = self.scheduler_max_timer_coalesce_ns {
            insert_runtime_value(
                &mut runtime,
                "scheduler",
                "maxTimerCoalesceNs",
                max_timer_coalesce_ns.into(),
            );
        }
        if let Some(preempt_interval_ns) = self.scheduler_preempt_interval_ns {
            insert_runtime_value(
                &mut runtime,
                "scheduler",
                "preemptIntervalNs",
                preempt_interval_ns.into(),
            );
        }
        if let Some(task_limit) = self.scheduler_task_limit {
            insert_runtime_value(&mut runtime, "scheduler", "taskLimit", task_limit.into());
        }

        // apply heap settings
        if let Some(growth_percent) = self.heap_local_growth_percent {
            insert_runtime_value(
                &mut runtime,
                "heap",
                "gc",
                heap_space_value("local", "growthPercent", growth_percent.into()),
            );
        }
        if let Some(memory_limit_bytes) = self.heap_local_memory_limit_bytes {
            insert_runtime_value(
                &mut runtime,
                "heap",
                "gc",
                heap_space_value("local", "memoryLimitBytes", memory_limit_bytes.into()),
            );
        }
        if let Some(max_bytes) = self.heap_local_hard_limit_bytes {
            insert_runtime_value(
                &mut runtime,
                "heap",
                "limit",
                heap_space_value("local", "maxBytes", max_bytes.into()),
            );
        }
        if let Some(growth_percent) = self.heap_shared_growth_percent {
            insert_runtime_value(
                &mut runtime,
                "heap",
                "gc",
                heap_space_value("shared", "growthPercent", growth_percent.into()),
            );
        }
        if let Some(memory_limit_bytes) = self.heap_shared_memory_limit_bytes {
            insert_runtime_value(
                &mut runtime,
                "heap",
                "gc",
                heap_space_value("shared", "memoryLimitBytes", memory_limit_bytes.into()),
            );
        }
        if let Some(max_bytes) = self.heap_shared_hard_limit_bytes {
            insert_runtime_value(
                &mut runtime,
                "heap",
                "limit",
                heap_space_value("shared", "maxBytes", max_bytes.into()),
            );
        }

        Some(ConfigPatch {
            path: "runtime".to_string(),
            value: Value::Object(runtime),
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

impl ExecutionModeArg {
    /// Return the scheduler config value implied by this execution mode.
    fn scheduler_value(self) -> Value {
        match self {
            Self::Fast => Value::String("parallel".into()),
            Self::Deterministic | Self::Record | Self::Replay => {
                Value::String("cooperative".into())
            }
        }
    }

    /// Return the trace config value implied by this execution mode.
    fn trace_value(self) -> Value {
        match self {
            Self::Fast | Self::Deterministic => Value::String("off".into()),
            Self::Record => Value::String("record".into()),
            Self::Replay => Value::String("replay".into()),
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

impl SchedulerPolicyArg {
    /// Return the scheduler policy config value.
    fn value(self) -> Value {
        match self {
            Self::Fifo => Value::String("fifo".into()),
            Self::Fair => Value::String("fair".into()),
            Self::WorkStealing => Value::String("workstealing".into()),
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

impl TimeModeArg {
    /// Return the clock source config value.
    fn value(self) -> Value {
        match self {
            Self::Host => Value::String("host".into()),
            Self::Virtual => Value::String("virtual".into()),
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

impl RandomModeArg {
    /// Return the randomness source config value.
    fn value(self) -> Value {
        match self {
            Self::Host => Value::String("host".into()),
            Self::Deterministic => Value::String("deterministic".into()),
        }
    }
}

/// Insert one nested runtime config value.
fn insert_runtime_value(runtime: &mut Map<String, Value>, section: &str, key: &str, value: Value) {
    let section = runtime
        .entry(section.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !section.is_object() {
        *section = Value::Object(Map::new());
    }

    if let Value::Object(section) = section {
        merge_json_value(section.entry(key.to_string()).or_insert(Value::Null), value);
    }
}

/// Build one heap space patch object.
fn heap_space_value(space: &str, key: &str, value: Value) -> Value {
    let mut field = Map::new();
    field.insert(key.to_string(), value);

    let mut object = Map::new();
    object.insert(space.to_string(), Value::Object(field));

    Value::Object(object)
}

/// Merge one json value into another.
fn merge_json_value(target: &mut Value, value: Value) {
    match (target, value) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, value) in source {
                merge_json_value(target.entry(key).or_insert(Value::Null), value);
            }
        }
        (target, value) => *target = value,
    }
}
