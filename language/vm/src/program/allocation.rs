use destack_heap as heap;
use destack_mir::ReferenceMap;

/// The allocation layout consumed by heap allocation opcodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AllocationLayout {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required allocation base alignment in bytes.
    pub alignment: usize,
    /// The exact heap reference map.
    pub reference_map: ReferenceMap,
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
    /// The allocation class for the targeted heap shape.
    pub class: heap::AllocationClass,
}

impl AllocationLayout {
    /// Return one borrowed heap allocation plan.
    #[inline(always)]
    pub(crate) fn plan(&self) -> heap::AllocationPlan<'_> {
        heap::AllocationPlan {
            byte_len: self.byte_len,
            alignment: self.alignment,
            reference_map: &self.reference_map,
            is_noscan: self.is_noscan,
            has_shared_reference: self.has_shared_reference,
        }
    }

    /// Return one heap allocation layout.
    #[inline(always)]
    pub(crate) fn heap_layout(&self) -> heap::AllocationLayout<'_> {
        heap::AllocationLayout {
            byte_len: self.byte_len,
            alignment: self.alignment,
            reference_map: &self.reference_map,
            is_noscan: self.is_noscan,
            has_shared_reference: self.has_shared_reference,
            class: self.class,
        }
    }
}
