use serde::{Deserialize, Serialize};
use tspp_core::Blob;
use tspp_memory::MemoryRange;
use tspp_program as program;
use tspp_serde::Reflect;
use tspp_source::Diagnostic;

use crate::runtime::RuntimeId;
use crate::world::Stop;

use super::FrameId;

/// Context selected for expression evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum EvaluationTarget {
    /// One captured execution Frame.
    Frame(FrameId),
    /// One Runtime module outside a Frame.
    Module {
        /// Selected Runtime.
        runtime_id: RuntimeId,
        /// Selected Program module.
        module_id: program::ModuleId,
    },
}

/// Mutation behavior for expression evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum EvaluationMode {
    /// Evaluate without retaining mutations.
    Inspect,
    /// Evaluate in a new forked World.
    Fork,
    /// Retain mutations in the selected World.
    Commit,
}

/// Resource limits for one expression evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EvaluationLimits {
    /// Maximum executed instructions.
    pub instructions: u64,
    /// Maximum bytes allocated by evaluation.
    pub allocation_bytes: u64,
    /// Maximum direct result bytes.
    pub result_bytes: u64,
}

/// Materialized representation of one evaluated TS++ value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum EvaluationRepresentation {
    /// Small canonical value bytes returned inline.
    Bytes(Vec<u8>),
    /// Live value retained in World memory.
    Memory(MemoryRange),
    /// Large immutable value retained as a Blob.
    Blob(Blob),
}

/// One evaluated TS++ value and its defining Program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EvaluationValue {
    /// Program required to interpret the value type.
    pub program: Blob,
    /// Program type of the value.
    pub ty: program::TypeId,
    /// Materialized value representation.
    pub representation: EvaluationRepresentation,
}

/// Outcome from one expression evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum EvaluationOutcome {
    /// Expression returned one value.
    Returned(EvaluationValue),
    /// Expression threw one value.
    Threw(EvaluationValue),
    /// Expression could not be compiled or admitted.
    Rejected(Vec<Diagnostic>),
    /// Expression execution reached a debugger stop.
    Stopped(Stop),
    /// Evaluation exceeded one resource limit.
    Exhausted,
    /// Evaluation was cancelled.
    Cancelled,
}
