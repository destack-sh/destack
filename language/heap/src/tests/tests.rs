use std::sync::Arc;

use crate::{Arena, Heap, HeapLimits, HeapOptions};

/// Create one shared arena for one explicit heap options set.
pub(super) fn test_arena(options: &HeapOptions) -> Arc<Arena> {
    Arc::new(
        Arena::try_new(options.page_bytes, options.arena_segment_bytes)
            .expect("checked heap options should build one arena"),
    )
}

/// One heap test harness.
pub(super) struct TestHeap {
    /// The heap under test.
    pub(super) heap: Heap,
}

impl TestHeap {
    /// Create one test heap with default options.
    pub(super) fn new() -> Self {
        Self {
            heap: Heap::new().expect("default heap should build"),
        }
    }

    /// Create one test heap with explicit options.
    pub(super) fn with_options(options: HeapOptions) -> Self {
        Self::with_limits_and_options(HeapLimits::default(), options)
    }

    /// Create one test heap with explicit limits and options.
    pub(super) fn with_limits_and_options(limits: HeapLimits, options: HeapOptions) -> Self {
        Self {
            heap: Heap::with_limits_and_options(limits, options)
                .expect("explicit heap options should build"),
        }
    }
}
