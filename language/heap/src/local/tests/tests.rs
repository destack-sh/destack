use crate::local::{Heap, HeapLimits, HeapOptions};

/// One heap test harness.
pub(crate) struct TestHeap {
    /// The heap under test.
    pub(crate) heap: Heap,
}

impl TestHeap {
    /// Create one test heap with default options.
    pub(crate) fn new() -> Self {
        Self {
            heap: Heap::new().expect("default heap should build"),
        }
    }

    /// Create one test heap with explicit options.
    pub(crate) fn with_options(options: HeapOptions) -> Self {
        Self::with_limits_and_options(HeapLimits::default(), options)
    }

    /// Create one test heap with explicit limits and options.
    pub(crate) fn with_limits_and_options(limits: HeapLimits, options: HeapOptions) -> Self {
        Self {
            heap: Heap::with_limits_and_options(limits, options)
                .expect("explicit heap options should build"),
        }
    }
}
