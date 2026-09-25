use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_mir as mir;
use tspp_serde::Reflect;

use super::Point;

/// One allocation site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationSite {
    /// The operation performing the allocation.
    pub point: Point,
    /// The storage space receiving the allocation.
    pub space: mir::Space,
    /// The type produced by the allocation.
    pub result_type: mir::TypeId,
    /// The type stored in the allocation.
    pub storage_type: mir::TypeId,
}

/// One addressable memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemorySite {
    /// The operation performing the access.
    pub point: Point,
    /// The memory access operation.
    pub access: mir::MemoryOperation,
    /// The addressed relative storage, or none for an absolute pointer.
    pub storage: Option<mir::Storage>,
    /// The loaded or stored value type.
    pub value_type: mir::TypeId,
}

/// One function call operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallSite {
    /// The operation performing the call.
    pub point: Point,
    /// The operation entered after normal completion when the call returns.
    pub resume: Option<Point>,
    /// The operation entered during panic unwinding when present.
    pub unwind: Option<Point>,
    /// Whether the call returns to its caller.
    pub mode: CallMode,
    /// The call dispatch mechanism.
    pub dispatch: mir::CallDispatch,
    /// The receiver storage space when present.
    pub space: Option<mir::Space>,
    /// The direct target when present.
    pub target: Option<mir::FunctionId>,
    /// The virtual or dynamic dispatch type when present.
    pub dispatch_type: Option<mir::TypeId>,
    /// The callable signature.
    pub signature: mir::TypeId,
}

/// One control flow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EdgeSite {
    /// The operation producing the transfer.
    pub source: Point,
    /// The operation entered after the transfer.
    pub target: Point,
}

/// One explicit profile counter operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CounterSite {
    /// The instrument name a user counter declares.
    pub name: Option<StringId>,
    /// The operation incrementing the counter.
    pub point: Point,
    /// The function-local counter.
    pub counter: mir::CounterId,
}

/// One explicit profile sample operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SampleSite {
    /// The instrument name a user sampler declares.
    pub name: Option<StringId>,
    /// The operation recording the sample.
    pub point: Point,
    /// The function-local sampler.
    pub sampler: mir::SamplerId,
    /// The sampled value type.
    pub value_type: mir::TypeId,
}

/// Call return mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CallMode {
    /// Return to the caller after the callee finishes.
    Return,
    /// Replace the caller frame with the callee frame.
    Tail,
}
