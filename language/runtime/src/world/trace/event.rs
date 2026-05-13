use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
use crate::host::HostError;
use crate::host::binding::{BindingEngine, BindingId, CodecId};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use crate::runtime::time::Instant;
use crate::runtime::{RuntimeId, RuntimeImage, WorkerId, WorkerImage};
use crate::world::Command;
use destack_vm as vm;
use std::collections::BTreeMap;
use std::sync::Arc;

/// One authoritative replay record.
// NOTE #Performance: trace entropy payloads stay inline for replay locality
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceRecord {
    /// One input that entered the world.
    Command(Command),
    /// One observed outcome that replay cannot derive.
    Outcome(Outcome),
    /// One retained or user-visible history anchor.
    Anchor(String),
}

/// One observed outcome that replay cannot derive.
// NOTE #Performance: trace entropy payloads stay inline for replay locality
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome {
    /// One virtual-world time advance outcome.
    TimeAdvance(Instant),
    /// Entropy outcome for time and random nondeterminism.
    Entropy(EntropyEvent),
    /// External binding call payload or result.
    BindingCall(BindingCallEvent),
    /// One runtime spawn outcome that must be replayed structurally.
    RuntimeSpawned {
        /// The created runtime identifier.
        runtime_id: RuntimeId,
        /// The created runtime name.
        runtime_name: String,
        /// The created runtime labels.
        runtime_labels: BTreeMap<String, String>,
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
        /// The created worker name.
        worker_name: String,
        /// The created worker labels.
        worker_labels: BTreeMap<String, String>,
        /// Captured worker metadata for the created worker.
        worker: Arc<WorkerImage>,
    },
}

/// One worker image paired with its spawn identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedWorkerImage {
    /// The created worker name.
    pub name: String,
    /// The created worker labels.
    pub labels: BTreeMap<String, String>,
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

/// Trace key for entropy routing in bindings and trace handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntropyKind {
    /// Monotonic clock sample.
    TimeReadMonotonic,
    /// Wall clock sample.
    TimeReadWall,
    /// Stream allocation event.
    RandomStreamCreate,
    /// Stream u64 sample event.
    RandomReadU64,
    /// Stream bytes sample event.
    RandomReadBytes,
}

/// Runtime subject metadata for one entropy event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntropySubject {
    /// Runtime identifier for this entropy event.
    pub runtime_id: RuntimeId,
    /// Worker identifier for this entropy event.
    pub worker_id: WorkerId,
    /// Binding identifier for this entropy event.
    pub binding_id: BindingId,
    /// Optional engine for this entropy event.
    pub engine: Option<BindingEngine>,
    /// Optional task identifier for this entropy event.
    pub task_id: Option<TaskId>,
    /// Optional microtask identifier for this entropy event.
    pub microtask_id: Option<MicrotaskId>,
}

/// Encoded trace error payload for deterministic trace.
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: keep trace errors simple until the payload model settles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TraceError {
    /// VM runtime error payload.
    Vm(vm::Error),
    /// Host runtime error payload.
    Host(HostError),
    /// Binding-not-found runtime error payload.
    BindingNotFound {
        /// Fully qualified binding name.
        name: String,
    },
    /// Policy-violation runtime error payload.
    PolicyViolation {
        /// Fully qualified binding name.
        name: String,
    },
    /// Action-violation runtime error payload.
    ActionDenied {
        /// Fully qualified binding name.
        name: String,
        /// Missing required action.
        action: String,
    },
    /// Affinity-violation runtime error payload.
    AffinityViolation {
        /// Fully qualified binding name.
        name: String,
        /// Required affinity classification.
        affinity: String,
    },
    /// Resource-not-found runtime error payload.
    ResourceNotFound {
        /// Resource id or handle.
        resource_id: u64,
        /// Optional resource kind.
        resource_kind: Option<String>,
    },
    /// Event-loop-idle runtime error payload.
    EventLoopIdle {
        /// Task identifier for the idle event.
        task_id: u64,
    },
    /// Trace-exhausted runtime error payload.
    TraceExhausted {
        /// Sequence number of the missing replay event.
        sequence: u64,
    },
    /// Trace-mismatch runtime error payload.
    TraceMismatch {
        /// Channel name that mismatched.
        name: String,
    },
    /// Trace-payload-unsupported runtime error payload.
    TracePayloadUnsupported {
        /// Binding name for this payload mismatch.
        name: String,
    },
    /// Trace-encode-failed runtime error payload.
    TraceEncodeFailed {
        /// Binding name for this payload encode failure.
        name: String,
    },
    /// Trace-decode-failed runtime error payload.
    TraceDecodeFailed {
        /// Binding name for this payload decode failure.
        name: String,
    },
    /// Internal runtime error payload.
    Internal {
        /// Runtime error message.
        message: String,
    },
}

impl From<&RuntimeError> for TraceError {
    fn from(error: &RuntimeError) -> Self {
        match error {
            RuntimeError::Vm(error) => Self::Vm(error.as_ref().clone()),
            RuntimeError::Host(error) => Self::Host(error.as_ref().clone()),
            RuntimeError::BindingNotFound { name } => Self::BindingNotFound { name: name.clone() },
            RuntimeError::PolicyViolation { name } => Self::PolicyViolation { name: name.clone() },
            RuntimeError::ActionDenied { name, action } => Self::ActionDenied {
                name: name.clone(),
                action: action.clone(),
            },
            RuntimeError::AffinityViolation { name, affinity } => Self::AffinityViolation {
                name: name.clone(),
                affinity: affinity.clone(),
            },
            RuntimeError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => Self::ResourceNotFound {
                resource_id: *resource_id,
                resource_kind: resource_kind.clone(),
            },
            RuntimeError::EventLoopIdle { task_id } => Self::EventLoopIdle { task_id: *task_id },
            RuntimeError::TraceExhausted { sequence } => Self::TraceExhausted {
                sequence: *sequence,
            },
            RuntimeError::TraceMismatch { name } => Self::TraceMismatch { name: name.clone() },
            RuntimeError::TracePayloadUnsupported { name } => {
                Self::TracePayloadUnsupported { name: name.clone() }
            }
            RuntimeError::TraceEncodeFailed { name } => {
                Self::TraceEncodeFailed { name: name.clone() }
            }
            RuntimeError::TraceDecodeFailed { name } => {
                Self::TraceDecodeFailed { name: name.clone() }
            }
            RuntimeError::Internal { message } => Self::Internal {
                message: message.clone(),
            },
            other => Self::Internal {
                message: other.message(),
            },
        }
    }
}

impl From<TraceError> for RuntimeError {
    fn from(error: TraceError) -> Self {
        match error {
            TraceError::Vm(error) => Self::Vm(Box::new(error)),
            TraceError::Host(error) => Self::Host(error.boxed()),
            TraceError::BindingNotFound { name } => Self::BindingNotFound { name },
            TraceError::PolicyViolation { name } => Self::PolicyViolation { name },
            TraceError::ActionDenied { name, action } => Self::ActionDenied { name, action },
            TraceError::AffinityViolation { name, affinity } => {
                Self::AffinityViolation { name, affinity }
            }
            TraceError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => Self::ResourceNotFound {
                resource_id,
                resource_kind,
            },
            TraceError::EventLoopIdle { task_id } => Self::EventLoopIdle { task_id },
            TraceError::TraceExhausted { sequence } => Self::TraceExhausted { sequence },
            TraceError::TraceMismatch { name } => Self::TraceMismatch { name },
            TraceError::TracePayloadUnsupported { name } => Self::TracePayloadUnsupported { name },
            TraceError::TraceEncodeFailed { name } => Self::TraceEncodeFailed { name },
            TraceError::TraceDecodeFailed { name } => Self::TraceDecodeFailed { name },
            TraceError::Internal { message } => Self::Internal { message },
        }
    }
}

impl From<TraceError> for Box<RuntimeError> {
    fn from(error: TraceError) -> Self {
        RuntimeError::from(error).boxed()
    }
}

/// Entropy event captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntropyEvent {
    /// Monotonic clock read event.
    TimeReadMonotonic {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Monotonic clock read outcome in nanoseconds.
        outcome: Result<u64, TraceError>,
    },
    /// Wall clock read event.
    TimeReadWall {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Wall clock read outcome in nanoseconds.
        outcome: Result<u64, TraceError>,
    },
    /// Deterministic stream allocation event.
    RandomStreamCreate {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Allocated stream identifier outcome.
        outcome: Result<RandomStreamId, TraceError>,
    },
    /// Deterministic stream u64 sample event.
    RandomReadU64 {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Random u64 sample outcome.
        outcome: Result<u64, TraceError>,
    },
    /// Deterministic stream bytes sample event.
    RandomReadBytes {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Requested byte count for this read.
        len: u32,
        /// Random bytes read outcome.
        outcome: Result<Vec<u8>, TraceError>,
    },
}

impl EntropyEvent {
    /// Return the trace key for this entropy event.
    pub const fn kind(&self) -> EntropyKind {
        match self {
            Self::TimeReadMonotonic { .. } => EntropyKind::TimeReadMonotonic,
            Self::TimeReadWall { .. } => EntropyKind::TimeReadWall,
            Self::RandomStreamCreate { .. } => EntropyKind::RandomStreamCreate,
            Self::RandomReadU64 { .. } => EntropyKind::RandomReadU64,
            Self::RandomReadBytes { .. } => EntropyKind::RandomReadBytes,
        }
    }

    /// Return the runtime subject for this entropy event.
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

/// External binding call event captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingCallEvent {
    /// Binding identifier from the registry.
    pub binding_id: BindingId,
    /// Codec identifier for the payload.
    pub codec: CodecId,
    /// Encoded payload for trace.
    pub payload: Vec<u8>,
}
