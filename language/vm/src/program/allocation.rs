use destack_heap as heap;
use destack_mir::TraceMap;

use super::{AllocationClassId, TraceMapId};

/// The allocation site consumed by heap allocation instructions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AllocationSite {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required allocation base alignment in bytes.
    pub alignment: usize,
    /// The exact heap trace map id.
    pub trace_map: TraceMapId,
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
    /// The allocation class id for the targeted heap shape.
    pub class: AllocationClassId,
}

impl AllocationSite {
    /// Return one borrowed heap allocation shape.
    #[inline(always)]
    pub(crate) fn shape<'a>(&self, trace_map: &'a TraceMap) -> heap::AllocationShape<'a> {
        heap::AllocationShape {
            byte_len: self.byte_len,
            alignment: self.alignment,
            trace_map,
            is_noscan: self.is_noscan,
            has_shared_reference: self.has_shared_reference,
        }
    }

    /// Return one heap allocation plan.
    #[inline(always)]
    pub(crate) fn heap_plan<'a>(
        &self,
        trace_map: &'a TraceMap,
        class: heap::AllocationClass,
    ) -> heap::AllocationPlan<'a> {
        heap::AllocationPlan {
            byte_len: self.byte_len,
            alignment: self.alignment,
            trace_map,
            is_noscan: self.is_noscan,
            has_shared_reference: self.has_shared_reference,
            class,
        }
    }
}
