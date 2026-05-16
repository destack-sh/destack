use destack_artifact::Runtime;
use serde::{Deserialize, Serialize};

use crate::{ConditionSet, ExecutionMode, ReplayPayloadMode};

use super::random::{RandomOptions, RandomSource};
use super::replay::ReplayOptions;
use super::time::{ClockSource, TimeOptions};
use super::{
    HeapOptions, HostOptions, PlatformOptions, RuntimeDiagnosticOptions, SchedulerMode,
    SchedulerOptions, TraceMode, TraceOptions, WorkerOptions,
};

/// Runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
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
