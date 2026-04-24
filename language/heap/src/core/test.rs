use destack_mir::ReferenceMap;

use crate::AllocationLayout;

/// One allocation layout used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestLayout {
    /// The exact payload byte length.
    pub(crate) byte_len: usize,
    /// The exact heap reference map.
    pub(crate) reference_map: ReferenceMap,
}

impl TestLayout {
    /// Return this test layout as heap allocation facts.
    pub(crate) fn allocation(&self) -> AllocationLayout<'_> {
        AllocationLayout::new(self.byte_len, &self.reference_map)
    }
}

/// Create test allocation layouts.
pub(crate) fn test_layouts(layouts: &[(usize, ReferenceMap)]) -> Vec<TestLayout> {
    layouts
        .iter()
        .map(|(byte_len, reference_map)| TestLayout {
            byte_len: *byte_len,
            reference_map: reference_map.clone(),
        })
        .collect()
}

/// Create one test allocation layout.
pub(crate) fn test_layout(byte_len: usize, reference_map: ReferenceMap) -> TestLayout {
    let mut layouts = test_layouts(&[(byte_len, reference_map)]);

    layouts.remove(0)
}
