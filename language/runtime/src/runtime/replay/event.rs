use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;
use crate::runtime::bindings::{BindingEngine, BindingId, CodecId};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use crate::runtime::time::WorldInstant;
use crate::runtime::world::WorldCommand;
use crate::runtime::{AgentId, RuntimeId};
use destack_vm as vm;

/// Event types recorded for deterministic replay.
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: replay entropy payloads are intentionally inline for now
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum ReplayEvent {
    /// Virtual world deadline selected for one world-level time advance.
    Tick(WorldInstant),
    /// Entropy event for time and random nondeterminism.
    Entropy(EntropyEvent),
    /// External binding call and result.
    BindingCall(BindingCallEvent),
    /// Application of one world command.
    WorldCommand(WorldCommand),
}

/// Replay key for entropy routing in bindings and replay handlers.
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
    /// Agent identifier for this entropy event.
    pub agent_id: AgentId,
    /// Binding identifier for this entropy event.
    pub binding_id: BindingId,
    /// Optional engine for this entropy event.
    pub engine: Option<BindingEngine>,
    /// Optional task identifier for this entropy event.
    pub task_id: Option<TaskId>,
    /// Optional microtask identifier for this entropy event.
    pub microtask_id: Option<MicrotaskId>,
}

/// Encoded replay error payload for deterministic replay.
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: keep replay errors simple until the payload model settles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReplayError {
    /// VM runtime error payload.
    Vm(vm::Error),
    /// Platform runtime error payload.
    Platform(PlatformError),
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
    /// Capability-violation runtime error payload.
    CapabilityViolation {
        /// Fully qualified binding name.
        name: String,
        /// Missing required capability.
        capability: String,
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
    /// Replay-log-exhausted runtime error payload.
    ReplayLogExhausted {
        /// Sequence number of the missing replay event.
        sequence: u64,
    },
    /// Replay-mismatch runtime error payload.
    ReplayMismatch {
        /// Channel name that mismatched.
        name: String,
    },
    /// Replay-payload-unsupported runtime error payload.
    ReplayPayloadUnsupported {
        /// Binding name for this payload mismatch.
        name: String,
    },
    /// Missing binding-call-context runtime error payload.
    BindingCallContextMissing,
    /// Replay-encode-failed runtime error payload.
    ReplayEncodeFailed {
        /// Binding name for this payload encode failure.
        name: String,
    },
    /// Replay-decode-failed runtime error payload.
    ReplayDecodeFailed {
        /// Binding name for this payload decode failure.
        name: String,
    },
    /// Internal runtime error payload.
    Internal {
        /// Runtime error message.
        message: String,
    },
}

impl From<&RuntimeError> for ReplayError {
    fn from(error: &RuntimeError) -> Self {
        match error {
            RuntimeError::Vm(error) => Self::Vm(error.as_ref().clone()),
            RuntimeError::Platform(error) => Self::Platform(error.as_ref().clone()),
            RuntimeError::BindingNotFound { name } => Self::BindingNotFound { name: name.clone() },
            RuntimeError::PolicyViolation { name } => Self::PolicyViolation { name: name.clone() },
            RuntimeError::CapabilityViolation { name, capability } => Self::CapabilityViolation {
                name: name.clone(),
                capability: capability.clone(),
            },
            RuntimeError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => Self::ResourceNotFound {
                resource_id: *resource_id,
                resource_kind: resource_kind.clone(),
            },
            RuntimeError::EventLoopIdle { task_id } => Self::EventLoopIdle { task_id: *task_id },
            RuntimeError::ReplayLogExhausted { sequence } => Self::ReplayLogExhausted {
                sequence: *sequence,
            },
            RuntimeError::ReplayMismatch { name } => Self::ReplayMismatch { name: name.clone() },
            RuntimeError::ReplayPayloadUnsupported { name } => {
                Self::ReplayPayloadUnsupported { name: name.clone() }
            }
            RuntimeError::BindingCallContextMissing => Self::BindingCallContextMissing,
            RuntimeError::ReplayEncodeFailed { name } => {
                Self::ReplayEncodeFailed { name: name.clone() }
            }
            RuntimeError::ReplayDecodeFailed { name } => {
                Self::ReplayDecodeFailed { name: name.clone() }
            }
            RuntimeError::Internal { message } => Self::Internal {
                message: message.clone(),
            },
        }
    }
}

impl From<ReplayError> for RuntimeError {
    fn from(error: ReplayError) -> Self {
        match error {
            ReplayError::Vm(error) => Self::Vm(Box::new(error)),
            ReplayError::Platform(error) => Self::Platform(error.boxed()),
            ReplayError::BindingNotFound { name } => Self::BindingNotFound { name },
            ReplayError::PolicyViolation { name } => Self::PolicyViolation { name },
            ReplayError::CapabilityViolation { name, capability } => {
                Self::CapabilityViolation { name, capability }
            }
            ReplayError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => Self::ResourceNotFound {
                resource_id,
                resource_kind,
            },
            ReplayError::EventLoopIdle { task_id } => Self::EventLoopIdle { task_id },
            ReplayError::ReplayLogExhausted { sequence } => Self::ReplayLogExhausted { sequence },
            ReplayError::ReplayMismatch { name } => Self::ReplayMismatch { name },
            ReplayError::ReplayPayloadUnsupported { name } => {
                Self::ReplayPayloadUnsupported { name }
            }
            ReplayError::BindingCallContextMissing => Self::BindingCallContextMissing,
            ReplayError::ReplayEncodeFailed { name } => Self::ReplayEncodeFailed { name },
            ReplayError::ReplayDecodeFailed { name } => Self::ReplayDecodeFailed { name },
            ReplayError::Internal { message } => Self::Internal { message },
        }
    }
}

impl From<ReplayError> for Box<RuntimeError> {
    fn from(error: ReplayError) -> Self {
        RuntimeError::from(error).boxed()
    }
}

/// Entropy event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntropyEvent {
    /// Monotonic clock read event.
    TimeReadMonotonic {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Monotonic clock read outcome in nanoseconds.
        outcome: Result<u64, ReplayError>,
    },
    /// Wall clock read event.
    TimeReadWall {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Wall clock read outcome in nanoseconds.
        outcome: Result<u64, ReplayError>,
    },
    /// Deterministic stream allocation event.
    RandomStreamCreate {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Allocated stream identifier outcome.
        outcome: Result<RandomStreamId, ReplayError>,
    },
    /// Deterministic stream u64 sample event.
    RandomReadU64 {
        /// Runtime subject metadata for this entropy event.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Random u64 sample outcome.
        outcome: Result<u64, ReplayError>,
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
        outcome: Result<Vec<u8>, ReplayError>,
    },
}

impl EntropyEvent {
    /// Return the replay key for this entropy event.
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

/// External binding call event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingCallEvent {
    /// Binding identifier from the registry.
    pub binding_id: BindingId,
    /// Codec identifier for the payload.
    pub codec: CodecId,
    /// Encoded payload for replay.
    pub payload: Vec<u8>,
}
