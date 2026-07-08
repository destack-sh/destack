use crate::host::binding::{BindingId, CodecId};
use crate::runtime::machine::Entry;
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::RunnableId;
use crate::runtime::time::Instant;
use crate::runtime::{RuntimeId, WorkerId};
use crate::world::Mutation;
use destack_program as program;
use serde::{Deserialize, Serialize};

use super::{TraceError, TraceSequence};

/// One authoritative replay payload.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trace {
    /// One mutation that entered the world.
    Mutation(Mutation),
    /// One runtime entrypoint call that entered the world.
    Entrypoint(EntrypointCall),
    /// One binding result that replay cannot derive.
    Binding(BindingTrace),
    /// One clock fact that replay cannot derive.
    Clock(ClockTrace),
    /// One random fact that replay cannot derive.
    Random(RandomTrace),
}

/// One ordered replay payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Trace sequence number.
    pub sequence: TraceSequence,
    /// Recorded replay payload.
    pub trace: Trace,
}

/// One replayable runtime entrypoint call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrypointCall {
    /// Runtime identifier that owns the entrypoint execution.
    pub runtime_id: RuntimeId,
    /// Replayable entrypoint reference.
    pub entry: Entry,
    /// Invocation arguments.
    pub args: Vec<program::Value>,
}

impl Trace {
    /// Return the stable trace name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Mutation(mutation) => mutation.name(),
            Self::Entrypoint(_) => "runtime.instance.entrypoint.run",
            Self::Binding(_) => "runtime.binding.call",
            Self::Clock(ClockTrace::Advance(_)) => "runtime.time.advance",
            Self::Clock(_) => "runtime.time.read",
            Self::Random(_) => "runtime.random.read",
        }
    }
}

impl TraceEntry {
    /// Return the stable trace entry name.
    pub fn name(&self) -> &'static str {
        self.trace.name()
    }
}

/// Runtime subject metadata for one nondeterministic trace fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntropySubject {
    /// Runtime identifier for this trace fact.
    pub runtime_id: RuntimeId,
    /// Worker identifier for this trace fact.
    pub worker_id: WorkerId,
    /// Binding identifier for this trace fact.
    pub binding_id: BindingId,
    /// Optional task identifier for this trace fact.
    pub task_id: Option<RunnableId>,
    /// Optional microtask identifier for this trace fact.
    pub microtask_id: Option<RunnableId>,
}

/// Clock fact captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClockTrace {
    /// Monotonic clock read.
    ReadMonotonic {
        /// Runtime subject metadata for this clock read.
        subject: EntropySubject,
        /// Monotonic clock read outcome in nanoseconds.
        outcome: Result<u64, TraceError>,
    },
    /// Wall clock read.
    ReadWall {
        /// Runtime subject metadata for this clock read.
        subject: EntropySubject,
        /// Wall clock read outcome in nanoseconds.
        outcome: Result<u64, TraceError>,
    },
    /// Runtime-controlled virtual time advance.
    Advance(Instant),
}

/// Random fact captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RandomTrace {
    /// Deterministic stream allocation.
    StreamCreate {
        /// Runtime subject metadata for this random fact.
        subject: EntropySubject,
        /// Allocated stream identifier outcome.
        outcome: Result<RandomStreamId, TraceError>,
    },
    /// Deterministic stream u64 read.
    ReadU64 {
        /// Runtime subject metadata for this random fact.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Random u64 sample outcome.
        outcome: Result<u64, TraceError>,
    },
    /// Deterministic stream bytes read.
    ReadBytes {
        /// Runtime subject metadata for this random fact.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Requested byte count for this read.
        len: u32,
        /// Random bytes read outcome.
        outcome: Result<Vec<u8>, TraceError>,
    },
}

impl ClockTrace {
    /// Return the runtime subject for this clock trace.
    pub const fn subject(&self) -> Option<EntropySubject> {
        match self {
            Self::ReadMonotonic { subject, .. } | Self::ReadWall { subject, .. } => Some(*subject),
            Self::Advance(_) => None,
        }
    }
}

impl RandomTrace {
    /// Return the runtime subject for this random trace.
    pub const fn subject(&self) -> EntropySubject {
        match self {
            Self::StreamCreate { subject, .. }
            | Self::ReadU64 { subject, .. }
            | Self::ReadBytes { subject, .. } => *subject,
        }
    }
}

/// Binding call captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingTrace {
    /// Binding identifier from the binding table.
    pub binding_id: BindingId,
    /// Codec identifier for encoded bytes.
    pub codec: CodecId,
    /// Encoded binding call bytes.
    pub bytes: Vec<u8>,
}
