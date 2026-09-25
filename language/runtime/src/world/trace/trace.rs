use crate::binding::CodecId;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::machine::Entry;
use crate::runtime::RuntimeId;
use crate::worker::{RunnableScope, WorkerId};
use crate::world::Mutation;
use crate::world::random::RandomStreamId;
use crate::world::time::Instant;
use serde::{Deserialize, Serialize};
use tspp_program as program;

use super::TraceSequence;

/// Result stored inside deterministic trace entries.
pub type TraceResult<T> = RuntimeResult<T>;

/// Encoded trace entry discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum TraceTag {
    /// World mutation trace.
    Mutation = 1,
    /// Runtime entrypoint trace.
    Entrypoint = 2,
    /// Binding call trace.
    Binding = 3,
    /// Clock trace.
    Clock = 4,
    /// Random trace.
    Random = 5,
}

/// One authoritative replay payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trace {
    /// One mutation that entered the world.
    Mutation(Box<Mutation>),
    /// One runtime entrypoint call that entered the world.
    Entrypoint(Box<EntrypointCall>),
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
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrypointCall {
    /// Runtime identifier that owns the entrypoint execution.
    pub runtime_id: RuntimeId,
    /// Replayable entrypoint reference.
    pub entry: Entry,
    /// Invocation arguments.
    pub args: Vec<program::Value>,
}

impl Clone for EntrypointCall {
    /// Share replay argument storage into one immutable trace copy.
    fn clone(&self) -> Self {
        Self {
            runtime_id: self.runtime_id,
            entry: self.entry.clone(),
            args: self.args.iter().map(program::Value::fork).collect(),
        }
    }
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

impl TraceTag {
    /// Decode one trace tag byte.
    pub(crate) fn from_byte(byte: u8) -> RuntimeResult<Self> {
        match byte {
            1 => Ok(Self::Mutation),
            2 => Ok(Self::Entrypoint),
            3 => Ok(Self::Binding),
            4 => Ok(Self::Clock),
            5 => Ok(Self::Random),
            _ => Err(RuntimeError::trace_mismatch("trace_tag".to_string()).boxed()),
        }
    }

    /// Return this trace tag as one byte.
    pub(crate) const fn byte(self) -> u8 {
        self as u8
    }

    /// Return the trace name used for diagnostics.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Mutation => "world",
            Self::Entrypoint => "entrypoint",
            Self::Binding => "binding",
            Self::Clock => "time",
            Self::Random => "random",
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
    pub binding_id: program::BindingId,
    /// Runnable scope for this trace fact.
    pub scope: RunnableScope,
}

/// Clock fact captured for trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClockTrace {
    /// Monotonic clock read.
    ReadMonotonic {
        /// Runtime subject metadata for this clock read.
        subject: EntropySubject,
        /// Monotonic clock read outcome in nanoseconds.
        outcome: TraceResult<u64>,
    },
    /// Wall clock read.
    ReadWall {
        /// Runtime subject metadata for this clock read.
        subject: EntropySubject,
        /// Wall clock read outcome in nanoseconds.
        outcome: TraceResult<u64>,
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
        outcome: TraceResult<RandomStreamId>,
    },
    /// Deterministic stream u64 read.
    ReadU64 {
        /// Runtime subject metadata for this random fact.
        subject: EntropySubject,
        /// Random stream identifier for this read.
        stream_id: RandomStreamId,
        /// Random u64 sample outcome.
        outcome: TraceResult<u64>,
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
        outcome: TraceResult<Vec<u8>>,
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
    pub binding_id: program::BindingId,
    /// Codec identifier for encoded bytes.
    pub codec: CodecId,
    /// Encoded binding call bytes.
    pub bytes: Vec<u8>,
}
