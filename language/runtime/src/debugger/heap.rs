use serde::{Deserialize, Serialize};
use tspp_memory::MemoryRange;
use tspp_program as program;
use tspp_serde::Reflect;

use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

use super::FrameId;

/// Stable identity of one managed heap Allocation at one Moment.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct AllocationId {
    /// Runtime that owns the Allocation.
    pub runtime_id: RuntimeId,
    /// Worker that owns a local Allocation, or absent for shared Allocations.
    pub worker_id: Option<WorkerId>,
    /// Logical allocation address in World memory.
    pub address: u64,
}

/// One managed heap Allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Allocation {
    /// Allocation identity.
    pub id: AllocationId,
    /// Exact allocation memory range.
    pub range: MemoryRange,
    /// Program allocation site when known.
    pub site_id: Option<program::AllocationSiteId>,
}

/// Stable identity of one heap Root at one Moment.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct RootId(
    /// Raw Root identifier.
    pub u64,
);

/// Source of one retained heap Root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum RootSource {
    /// Program global storage.
    Global {
        /// Owning Runtime.
        runtime_id: RuntimeId,
        /// Program global.
        global_id: program::GlobalId,
    },
    /// Captured execution Frame.
    Frame {
        /// Owning Frame.
        frame_id: FrameId,
        /// Byte offset inside the Frame.
        offset: u32,
    },
    /// Runtime host state.
    Host,
}

/// One retained heap Root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Root {
    /// Root identity.
    pub id: RootId,
    /// Root source.
    pub source: RootSource,
    /// Referenced Allocation.
    pub allocation_id: AllocationId,
}

/// One outgoing heap Reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Reference {
    /// Byte offset inside the source Allocation.
    pub offset: u64,
    /// Referenced Allocation.
    pub allocation_id: AllocationId,
}
