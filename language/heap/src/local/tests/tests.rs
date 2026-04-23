use std::sync::Arc;

use crate::allocator::Allocator;
use crate::local::{Heap, HeapLimits, HeapOptions};
use destack_mir::LayoutTable;

/// One heap test harness.
pub(crate) struct TestHeap {
    /// The heap under test.
    pub(crate) heap: Heap,
}

impl TestHeap {
    /// Create one test heap with default options.
    pub(crate) fn new() -> Self {
        let options = HeapOptions::local();
        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
                .expect("default allocator should build"),
        );

        Self {
            heap: Heap::with_allocator_limits_layouts_and_options(
                allocator,
                Arc::new(LayoutTable::new()),
                HeapLimits::default(),
                options,
            )
            .expect("default heap should build"),
        }
    }

    /// Create one test heap with explicit options.
    pub(crate) fn with_options(options: HeapOptions) -> Self {
        Self::with_limits_and_options(HeapLimits::default(), options)
    }

    /// Create one test heap with explicit limits and options.
    pub(crate) fn with_limits_and_options(limits: HeapLimits, options: HeapOptions) -> Self {
        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
                .expect("explicit allocator should build"),
        );

        Self {
            heap: Heap::with_allocator_limits_layouts_and_options(
                allocator,
                Arc::new(LayoutTable::new()),
                limits,
                options,
            )
            .expect("explicit heap options should build"),
        }
    }

    /// Create one test heap with one explicit layout table.
    pub(crate) fn with_limits_and_layout_table(
        limits: HeapLimits,
        options: HeapOptions,
        layouts: Arc<LayoutTable>,
    ) -> Self {
        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
                .expect("explicit allocator should build"),
        );

        Self {
            heap: Heap::with_allocator_limits_layouts_and_options(
                allocator, layouts, limits, options,
            )
            .expect("explicit heap layouts should build"),
        }
    }
}
