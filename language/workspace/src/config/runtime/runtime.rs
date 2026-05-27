use std::collections::BTreeMap;

use destack_artifact::Runtime;
use serde::{Deserialize, Serialize};

use crate::{ConditionSet, ExecutionMode, ReplayPayloadMode};

use super::clock::ClockOptions;
use super::random::RandomOptions;
use super::{
    ExecutionOptions, HeapOptions, HostOptions, RuntimeDiagnosticOptions, TraceOptions,
    WorkerOptions,
};

/// Runtime identity used for topology and policy selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIdentityOptions {
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Stable runtime labels for topology and policy selection.
    pub labels: BTreeMap<String, String>,
}

/// Runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOptions {
    /// Runtime implementation family.
    pub runtime: Runtime,
    /// Runtime topology and policy identity.
    pub identity: RuntimeIdentityOptions,
    /// Active source graph conditions for runtime policy selection.
    #[serde(default)]
    pub conditions: ConditionSet,
    /// Runtime execution configuration.
    pub execution: ExecutionOptions,
    /// Runtime worker configuration.
    pub worker: WorkerOptions,
    /// Runtime clock seed configuration.
    pub clock: ClockOptions,
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
}

impl RuntimeOptions {
    /// Return the configured execution mode.
    pub fn execution_mode(&self) -> ExecutionMode {
        self.execution.mode
    }

    /// Return the configured trace payload policy.
    pub fn replay_payload_mode(&self) -> ReplayPayloadMode {
        self.trace.payload
    }

    /// Return the configured trace chunk size in megabytes.
    pub fn trace_chunk_size_mb(&self) -> Option<u64> {
        self.trace.chunk_size_mb
    }
}
