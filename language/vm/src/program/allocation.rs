use destack_heap::{
    self as heap, AllocationSite as HeapAllocationSite,
    SmallAllocationSite as HeapSmallAllocationSite,
};
use destack_mir::{TraceId, TraceMap};

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
