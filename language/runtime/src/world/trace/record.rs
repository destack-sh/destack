use std::collections::BTreeMap;
use std::sync::Arc;

use crate::host::binding::{BindingId, CodecId};
use crate::runtime::engine::Entry;
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use crate::runtime::time::Instant;
use crate::runtime::{RuntimeId, RuntimeImage, WorkerId, WorkerImage};
use crate::world::{Entity, Mutation};
use serde::{Deserialize, Serialize};

use super::TraceError;

/// One authoritative replay record.
// NOTE #Performance: trace entropy payloads stay inline for replay locality
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceRecord {
    /// One structural mutation that entered the world.
    Mutation(Mutation),
    /// One runtime entrypoint call that entered the world.
    Entrypoint(EntrypointCall),
    /// One observed outcome that replay cannot derive.
    Outcome(Outcome),
    /// One retained or user-visible history label.
    Label(String),
}

/// One replayable runtime entrypoint call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrypointCall {
    /// Runtime identifier that owns the entrypoint execution.
    pub runtime_id: RuntimeId,
    /// Replayable entrypoint reference.
    pub entry: Entry,
    /// Invocation arguments.
    pub args: Vec<destack_engine::Value>,
}

/// One observed outcome that replay cannot derive.
// NOTE #Performance: trace entropy payloads stay inline for replay locality
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome {
    /// One virtual-world time advance outcome.
    TimeAdvance(Instant),
    /// Entropy sample for time and random nondeterminism.
    Entropy(EntropySample),
    /// External binding call payload or result.
    BindingCall(BindingCall),
    /// One runtime spawn outcome that must be replayed structurally.
    RuntimeSpawned {
        /// The created runtime identifier.
        runtime_id: RuntimeId,
        /// The created runtime topology entity.
        runtime_entity: Entity,
        /// Captured runtime metadata for the created runtime.
        runtime: Arc<RuntimeImage>,
        /// Captured workers keyed by worker identifier.
        workers: BTreeMap<WorkerId, SpawnedWorkerImage>,
    },
    /// One worker spawn outcome that must be replayed structurally.
    WorkerSpawned {
        /// The owning runtime identifier.
        runtime_id: RuntimeId,
        /// The created worker identifier.
        worker_id: WorkerId,
        /// The created worker topology entity.
        worker_entity: Entity,
        /// Captured worker metadata for the created worker.
        worker: Arc<WorkerImage>,
    },
}

/// One worker image paired with its spawn identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedWorkerImage {
    /// The created worker topology entity.
    pub entity: Entity,
    /// The captured worker payload.
    pub image: Arc<WorkerImage>,
}

impl Outcome {
    /// Return the stable outcome name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::TimeAdvance(_) => "runtime.time.advance",
            Self::Entropy(_) => "runtime.random.entropy",
            Self::BindingCall(_) => "runtime.binding.call",
            Self::RuntimeSpawned { .. } => "runtime.instance.spawned",
            Self::WorkerSpawned { .. } => "runtime.worker.spawned",
        }
    }
}

/// Runtime subject metadata for one entropy sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntropySubject {
    /// Runtime identifier for this entropy sample.
    pub runtime_id: RuntimeId,
    /// Worker identifier for this entropy sample.
    pub worker_id: WorkerId,
    /// Binding identifier for this entropy sample.
    pub binding_id: BindingId,
    /// Optional task identifier for this entropy sample.
    pub task_id: Option<TaskId>,
    /// Optional microtask identifier for this entropy sample.
    pub microtask_id: Option<MicrotaskId>,
}

/// Entropy sample captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntropySample {
    /// Monotonic clock read sample.
    TimeReadMonotonic {
        /// Runtime subject metadata for this entropy sample.
        subject: EntropySubject,
        /// Monotonic clock read outcome in nanoseconds.
        outcome: Result<u64, TraceError>,
    },
    /// Wall clock read sample.
    TimeReadWall {
        /// Runtime subject metadata for this entropy sample.
        subject: EntropySubject,
        /// Wall clock read outcome in nanoseconds.
        outcome: Result<u64, TraceError>,
    },
    /// Deterministic stream allocation sample.
    RandomStreamCreate {
        /// Runtime subject metadata for this entropy sample.
        subject: EntropySubject,
        /// Allocated stream identifier outcome.
        outcome: Result<RandomStreamId, TraceError>,
    },
    /// Deterministic stream u64 sample.
    RandomReadU64 {
        /// Runtime subject metadata for this entropy sample.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Random u64 sample outcome.
        outcome: Result<u64, TraceError>,
    },
    /// Deterministic stream bytes sample.
    RandomReadBytes {
        /// Runtime subject metadata for this entropy sample.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Requested byte count for this read.
        len: u32,
        /// Random bytes read outcome.
        outcome: Result<Vec<u8>, TraceError>,
    },
}

impl EntropySample {
    /// Return the runtime subject for this entropy sample.
    pub const fn subject(&self) -> EntropySubject {
        match self {
            Self::TimeReadMonotonic { subject, .. }
            | Self::TimeReadWall { subject, .. }
            | Self::RandomStreamCreate { subject, .. }
            | Self::RandomReadU64 { subject, .. }
            | Self::RandomReadBytes { subject, .. } => *subject,
        }
    }
}

/// External binding call captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingCall {
    /// Binding identifier from the registry.
    pub binding_id: BindingId,
    /// Codec identifier for the payload.
    pub codec: CodecId,
    /// Encoded payload for trace.
    pub payload: Vec<u8>,
}
