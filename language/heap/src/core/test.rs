use destack_mir::ReferenceMap;

use crate::AllocationPlan;

/// One allocation layout used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestLayout {
    /// The exact payload byte length.
    pub(crate) byte_len: usize,
    /// The required allocation base alignment in bytes.
    pub(crate) alignment: usize,
    /// The exact heap reference map.
    pub(crate) reference_map: ReferenceMap,
}

impl TestLayout {
    /// Return this test layout as resolved heap allocation facts.
    pub(crate) fn allocation(&self) -> AllocationPlan<'_> {
        AllocationPlan::new(self.byte_len, self.alignment, &self.reference_map)
    }
}

/// Create test allocation layouts.
pub(crate) fn test_layouts(layouts: &[(usize, ReferenceMap)]) -> Vec<TestLayout> {
    layouts
        .iter()
        .map(|(byte_len, reference_map)| TestLayout {
            byte_len: *byte_len,
            alignment: 1,
            reference_map: reference_map.clone(),
        })
        .collect()
}

/// Create one test allocation layout.
pub(crate) fn test_layout(byte_len: usize, reference_map: ReferenceMap) -> TestLayout {
    let mut layouts = test_layouts(&[(byte_len, reference_map)]);

    layouts.remove(0)
}

/// Create one aligned test allocation layout.
pub(crate) fn test_aligned_layout(
    byte_len: usize,
    alignment: usize,
    reference_map: ReferenceMap,
) -> TestLayout {
    TestLayout {
        byte_len,
        alignment,
        reference_map,
    }
}
