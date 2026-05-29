use destack_heap::{
    self as heap, AllocationSite as HeapAllocationSite,
    SmallAllocationSite as HeapSmallAllocationSite,
};
use destack_mir::{TraceId, TraceMap};

use super::{AllocationSiteId, Edge, SliceProjectionId};

/// The allocation site consumed by heap allocation instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllocationSite {
    /// The heap-ready allocation site.
    pub heap: HeapAllocationSite,
    /// The exact heap trace map id.
    pub trace_map: TraceId,
}

/// The small allocation site consumed by small heap allocation instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SmallAllocationSite {
    /// The full heap-ready allocation site for cold allocation.
    pub heap: HeapAllocationSite,
    /// The heap-ready small allocation site for cursor reservation.
    pub small: HeapSmallAllocationSite,
    /// The exact heap trace map id.
    pub trace_map: TraceId,
}

/// Branching allocation consumed by fallible heap allocation instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllocationBranch {
    /// The destination frame offset.
    pub destination: u32,
    /// The allocation site.
    pub allocation: AllocationSiteId,
    /// The success edge.
    pub success: Edge,
    /// The failure edge.
    pub failure: Edge,
}

/// Branching slice allocation consumed by fallible slice allocation instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SliceAllocationBranch {
    /// The destination frame offset.
    pub destination: u32,
    /// The length word frame offset.
    pub length: u32,
    /// The backing element allocation site.
    pub element: AllocationSiteId,
    /// The slice descriptor projection.
    pub access: SliceProjectionId,
    /// The success edge.
    pub success: Edge,
    /// The failure edge.
    pub failure: Edge,
}

impl AllocationSite {
    /// Return one borrowed heap allocation shape.
    #[inline(always)]
    pub(crate) fn shape<'a>(&self, trace_map: &'a TraceMap) -> heap::AllocationShape<'a> {
        let trace_id = if self.heap.is_noscan {
            None
        } else {
            Some(self.trace_map)
        };

        heap::AllocationShape {
            byte_len: self.heap.byte_len,
            alignment: self.heap.alignment,
            trace_id,
            trace_map,
            is_noscan: self.heap.is_noscan,
            has_shared_reference: self.heap.has_shared_reference,
        }
    }
}
