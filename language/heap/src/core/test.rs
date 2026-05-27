use destack_mir::TraceMap;

use crate::AllocationShape;

/// One allocation plan used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestLayout {
    /// The exact payload byte length.
    pub(crate) byte_len: usize,
    /// The required allocation base alignment in bytes.
    pub(crate) alignment: usize,
    /// The exact heap trace map.
    pub(crate) trace_map: TraceMap,
}

impl TestLayout {
    /// Return this test layout as resolved heap allocation facts.
    pub(crate) fn allocation(&self) -> AllocationShape<'_> {
        AllocationShape::new(self.byte_len, self.alignment, None, &self.trace_map)
    }
}

/// Create test allocation plans.
pub(crate) fn test_layouts(layouts: &[(usize, TraceMap)]) -> Vec<TestLayout> {
    layouts
        .iter()
        .map(|(byte_len, trace_map)| TestLayout {
            byte_len: *byte_len,
            alignment: 1,
            trace_map: trace_map.clone(),
        })
        .collect()
}

/// Create one test allocation plan.
pub(crate) fn test_layout(byte_len: usize, trace_map: TraceMap) -> TestLayout {
    let mut layouts = test_layouts(&[(byte_len, trace_map)]);

    layouts.remove(0)
}

/// Create one aligned test allocation plan.
pub(crate) fn test_aligned_layout(
    byte_len: usize,
    alignment: usize,
    trace_map: TraceMap,
) -> TestLayout {
    TestLayout {
        byte_len,
        alignment,
        trace_map,
    }
}
