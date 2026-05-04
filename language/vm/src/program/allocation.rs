use destack_heap as heap;
use destack_mir::ReferenceMap;

use super::{AllocationClassId, ReferenceMapId};

/// The allocation layout consumed by heap allocation instructions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AllocationLayout {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required allocation base alignment in bytes.
    pub alignment: usize,
    /// The exact heap reference map id.
    pub reference_map: ReferenceMapId,
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
    /// The allocation class id for the targeted heap shape.
    pub class: AllocationClassId,
}

impl AllocationLayout {
    /// Return one borrowed heap allocation plan.
    #[inline(always)]
    pub(crate) fn plan<'a>(&self, reference_map: &'a ReferenceMap) -> heap::AllocationPlan<'a> {
        heap::AllocationPlan {
            byte_len: self.byte_len,
            alignment: self.alignment,
            reference_map,
            is_noscan: self.is_noscan,
            has_shared_reference: self.has_shared_reference,
        }
    }

    /// Return one heap allocation layout.
    #[inline(always)]
    pub(crate) fn heap_layout<'a>(
        &self,
        reference_map: &'a ReferenceMap,
        class: heap::AllocationClass,
    ) -> heap::AllocationLayout<'a> {
        heap::AllocationLayout {
            byte_len: self.byte_len,
            alignment: self.alignment,
            reference_map,
            is_noscan: self.is_noscan,
            has_shared_reference: self.has_shared_reference,
            class,
        }
    }
}
