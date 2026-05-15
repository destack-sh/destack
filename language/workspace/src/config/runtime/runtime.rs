use destack_artifact::Runtime;
use serde::{Deserialize, Serialize};

use crate::{ConditionSet, ExecutionMode, ReplayPayloadMode};

use super::random::{RandomOptions, RandomOptionsJson, RandomSource};
use super::replay::ReplayOptions;
use super::time::{ClockSource, TimeOptions, TimeOptionsJson};
use super::{
    HeapOptions, HeapOptionsJson, HostOptions, HostOptionsJson, PlatformOptions,
    PlatformOptionsJson, RuntimeDiagnosticOptions, RuntimeDiagnosticOptionsJson, SchedulerMode,
    SchedulerOptions, SchedulerOptionsJson, TraceMode, TraceOptions, TraceOptionsJson,
    WorkerOptions, WorkerOptionsJson,
};

/// Runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeOptions {
    /// Runtime implementation family.
    pub runtime: Runtime,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Active source graph conditions for runtime policy selection.
    #[serde(default)]
    pub conditions: ConditionSet,
    /// Runtime scheduler configuration.
    pub scheduler: SchedulerOptions,
    /// Runtime worker configuration.
    pub worker: WorkerOptions,
    /// Runtime clock source configuration.
    pub time: TimeOptions,
    /// Runtime randomness source configuration.
    pub random: RandomOptions,
    /// Runtime trace configuration.
    pub trace: TraceOptions,
    /// Runtime heap configuration.
    pub heap: HeapOptions,
    /// Runtime diagnostics configuration.
    pub diagnostic: RuntimeDiagnosticOptions,
    /// Runtime host module defaults.
    pub host: HostOptions,
    /// Platform-specific host runtime overrides.
    pub platform: PlatformOptions,
}

impl RuntimeOptions {
    /// Apply one collapsed execution summary onto the runtime planes.
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        // scheduler mode
        self.scheduler.mode = match mode {
            ExecutionMode::Fast => SchedulerMode::Parallel,
            ExecutionMode::Deterministic | ExecutionMode::Record | ExecutionMode::Replay => {
                SchedulerMode::Cooperative
            }
        };

        // trace mode
        self.trace.mode = match mode {
            ExecutionMode::Fast | ExecutionMode::Deterministic => TraceMode::Off,
            ExecutionMode::Record => TraceMode::Record,
            ExecutionMode::Replay => TraceMode::Replay,
        };

        // replay uses virtual facts supplied by trace playback
        if mode == ExecutionMode::Replay {
            self.time.source = ClockSource::Virtual;
            self.random.source = RandomSource::Deterministic;
        }
    }

    /// Apply one collapsed time summary onto the runtime planes.
    pub fn set_clock_source(&mut self, source: ClockSource) {
        self.time.source = source;
    }

    /// Apply one collapsed randomness summary onto the runtime planes.
    pub fn set_random_source(&mut self, source: RandomSource) {
        self.random.source = source;
    }

    /// Return the collapsed execution summary.
    pub fn execution_mode(&self) -> ExecutionMode {
        // trace replay is always authoritative
        if self.trace.mode == TraceMode::Replay {
            return ExecutionMode::Replay;
        }

        // trace recording selects record execution
        if self.trace.mode == TraceMode::Record {
            return ExecutionMode::Record;
        }

        // cooperative worlds collapse to deterministic execution
        if self.scheduler.mode == SchedulerMode::Cooperative {
            return ExecutionMode::Deterministic;
        }

        ExecutionMode::Fast
    }

    /// Return the collapsed time-source summary.
    pub fn clock_source(&self) -> ClockSource {
        self.time.source
    }

    /// Return the collapsed randomness summary.
    pub fn random_source(&self) -> RandomSource {
        self.random.source
    }

    /// Return the configured trace payload policy.
    pub fn replay_payload_mode(&self) -> ReplayPayloadMode {
        self.trace.payload
    }

    /// Return the configured trace chunk size in megabytes.
    pub fn trace_chunk_size_mb(&self) -> Option<u64> {
        self.trace.chunk_size_mb
    }

    /// Return derived clock configuration for runtime internals.
    pub fn time_options(&self) -> TimeOptions {
        self.time.clone()
    }

    /// Return derived randomness configuration for runtime internals.
    pub fn random_options(&self) -> RandomOptions {
        self.random.clone()
    }

    /// Return derived trace-storage configuration for runtime internals.
    pub fn replay_options(&self) -> ReplayOptions {
        ReplayOptions {
            path: self.trace.path.clone(),
            template: self.trace.template.clone(),
            chunk_size_mb: self.trace.chunk_size_mb,
            payload: self.trace.payload,
        }
    }

    /// Return the configured scheduler options.
    pub fn scheduler_options(&self) -> &SchedulerOptions {
        &self.scheduler
    }
}

/// Runtime config JSON.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuntimeConfigJson {
    /// Runtime implementation shorthand.
    Runtime(Runtime),
    /// Full runtime configuration object.
    Options(Box<RuntimeOptionsJson>),
}

impl Default for RuntimeConfigJson {
    fn default() -> Self {
        Self::Options(Box::default())
    }
}

impl RuntimeConfigJson {
    /// Return this runtime config as object form.
    pub fn as_options_json(&self) -> RuntimeOptionsJson {
        match self {
            Self::Runtime(runtime) => RuntimeOptionsJson {
                runtime: Some(*runtime),
                ..RuntimeOptionsJson::default()
            },
            Self::Options(options) => options.as_ref().clone(),
        }
    }
}

pub(crate) fn runtime_options_from_json(json: Option<&RuntimeConfigJson>) -> RuntimeOptions {
    runtime_options_with_base(&RuntimeOptions::default(), json)
}

/// Derive runtime options from a base set of options plus overrides.
pub(crate) fn runtime_options_with_base(
    base: &RuntimeOptions,
    overrides: Option<&RuntimeConfigJson>,
) -> RuntimeOptions {
    // start from the base options
    let mut options = base.clone();

    // apply overrides when present
    if let Some(overrides) = overrides {
        overrides.as_options_json().apply_to(&mut options);
    }

    options
}

/// Runtime options (object form).
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOptionsJson {
    /// Runtime implementation family.
    pub runtime: Option<Runtime>,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime scheduler configuration.
    pub scheduler: Option<SchedulerOptionsJson>,
    /// Runtime worker configuration.
    pub worker: Option<WorkerOptionsJson>,
    /// Runtime clock source configuration.
    pub time: Option<TimeOptionsJson>,
    /// Runtime randomness source configuration.
    pub random: Option<RandomOptionsJson>,
    /// Runtime trace configuration.
    pub trace: Option<TraceOptionsJson>,
    /// Runtime heap configuration.
    pub heap: Option<HeapOptionsJson>,
    /// Runtime diagnostics configuration.
    pub diagnostic: Option<RuntimeDiagnosticOptionsJson>,
    /// Runtime host module defaults.
    pub host: Option<HostOptionsJson>,
    /// Platform-specific host runtime overrides.
    pub platform: Option<PlatformOptionsJson>,
}

impl RuntimeOptionsJson {
    /// Inherit unset runtime settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.runtime.is_none() {
            self.runtime = parent.runtime;
        }
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if let Some(scheduler) = &mut self.scheduler {
            if let Some(parent_scheduler) = &parent.scheduler {
                scheduler.extend_from(parent_scheduler);
            }
        } else {
            self.scheduler = parent.scheduler.clone();
        }
        if let Some(worker) = &mut self.worker {
            if let Some(parent_worker) = &parent.worker {
                worker.extend_from(parent_worker);
            }
        } else {
            self.worker = parent.worker.clone();
        }
        if let Some(time) = &mut self.time {
            if let Some(parent_time) = &parent.time {
                time.extend_from(parent_time);
            }
        } else {
            self.time = parent.time.clone();
        }
        if let Some(random) = &mut self.random {
            if let Some(parent_random) = &parent.random {
                random.extend_from(parent_random);
            }
        } else {
            self.random = parent.random.clone();
        }
        if let Some(trace) = &mut self.trace {
            if let Some(parent_trace) = &parent.trace {
                trace.extend_from(parent_trace);
            }
        } else {
            self.trace = parent.trace.clone();
        }
        if let Some(heap) = &mut self.heap {
            if let Some(parent_heap) = &parent.heap {
                heap.extend_from(parent_heap);
            }
        } else {
            self.heap = parent.heap.clone();
        }
        if self.diagnostic.is_none() {
            self.diagnostic = parent.diagnostic.clone();
        }
        if let Some(host) = &mut self.host {
            if let Some(parent_host) = &parent.host {
                host.extend_from(parent_host);
            }
        } else {
            self.host = parent.host.clone();
        }
        if self.platform.is_none() {
            self.platform = parent.platform.clone();
        }
    }

    /// Apply runtime option overrides to a base set of options.
    pub fn apply_to(&self, options: &mut RuntimeOptions) {
        // runtime
        if let Some(runtime) = self.runtime {
            options.runtime = runtime;
        }

        // runtime identity
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }

        // apply runtime planes
        if let Some(scheduler) = &self.scheduler {
            scheduler.apply_to(&mut options.scheduler);
        }

        if let Some(worker) = &self.worker {
            worker.apply_to(&mut options.worker);
        }

        if let Some(time) = &self.time {
            time.apply_to(&mut options.time);
        }

        if let Some(random) = &self.random {
            random.apply_to(&mut options.random);
        }

        if let Some(trace) = &self.trace {
            trace.apply_to(&mut options.trace);
        }

        // apply heap overrides
        if let Some(heap) = &self.heap {
            heap.apply_to(&mut options.heap);
        }

        // apply diagnostic overrides
        if let Some(diagnostic) = &self.diagnostic {
            diagnostic.apply_to(&mut options.diagnostic);
        }

        // apply host module defaults
        if let Some(host) = &self.host {
            host.apply_to(&mut options.host);
        }

        // apply platform overrides
        if let Some(platform) = &self.platform {
            platform.apply_to(&mut options.platform);
        }
    }
}
