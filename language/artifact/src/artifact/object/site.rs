use destack_mir as mir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::Point;

/// One heap allocation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationSite {
    /// The operation performing the allocation.
    pub point: Point,
    /// The allocation operation.
    pub operation: AllocationOperation,
    /// The byte initialization mode.
    pub initialization: AllocationInitialization,
    /// The storage space receiving the allocation.
    pub space: mir::Space,
    /// The type produced by the allocation.
    pub result_type: mir::TypeId,
    /// The type stored in the allocation.
    pub storage_type: mir::TypeId,
    /// The physical storage layout.
    pub storage_layout: mir::LayoutId,
}

/// One addressable memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemorySite {
    /// The operation performing the access.
    pub point: Point,
    /// The memory access operation.
    pub access: mir::MemoryOperation,
    /// The accessed storage space.
    pub space: mir::Space,
    /// The loaded or stored value type.
    pub value_type: mir::TypeId,
}

/// One function call operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallSite {
    /// The operation performing the call.
    pub point: Point,
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
    /// The callable signature type.
    pub signature_type: mir::TypeId,
}

/// One control flow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EdgeSite {
    /// The operation producing the transfer.
    pub source: Point,
    /// The operation entered after the transfer.
    pub target: Point,
}

/// One continuation suspension operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SuspensionSite {
    /// The operation suspending the function.
    pub point: Point,
    /// The operation entered by normal resume.
    pub resume: Point,
    /// The operation entered by panic unwinding when present.
    pub unwind: Option<Point>,
    /// The object-local frame state index.
    pub frame_state: u32,
    /// The type yielded to the coroutine owner.
    pub yielded_type: mir::TypeId,
}

/// One explicit profile counter operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CounterSite {
    /// The operation incrementing the counter.
    pub point: Point,
    /// The function-local counter.
    pub counter: mir::CounterId,
}

/// One explicit profile sample operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SampleSite {
    /// The operation recording the sample.
    pub point: Point,
    /// The function-local sampler.
    pub sampler: mir::SamplerId,
    /// The sampled value type.
    pub value_type: mir::TypeId,
}

/// Heap allocation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum AllocationOperation {
    /// Allocate one typed value.
    Value,
    /// Allocate repeated typed storage.
    Slice,
}

/// Allocation byte initialization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum AllocationInitialization {
    /// Initialize allocated bytes to zero.
    Zeroed,
    /// Leave allocated bytes uninitialized.
    Uninit,
}

/// Call continuation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CallMode {
    /// Return to the caller after the callee finishes.
    Return,
    /// Replace the caller frame with the callee frame.
    Tail,
}
