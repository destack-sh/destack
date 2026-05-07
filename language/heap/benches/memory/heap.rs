use std::sync::Arc;

use destack_heap::{
    Allocator, Heap, HeapLimits, HeapOptions, SharedAllocator, SharedGcWorker, SharedHeap,
    SharedHeapLimits,
};

/// One shared heap with one worker-local allocator.
pub(crate) struct SharedWorkerHeap {
    /// The shared heap under test.
    pub(crate) heap: SharedHeap,
    /// The worker-local allocator cache.
    pub(crate) allocator: SharedAllocator,
    /// The shared collector worker used by this worker heap.
    pub(crate) worker: SharedGcWorker,
}

/// Build one local heap suitable for allocation benchmarks.
pub(crate) fn local_heap() -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("heap should build")
}

/// Build one shared heap and worker-local allocator.
pub(crate) fn shared_worker_heap() -> SharedWorkerHeap {
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");

    let allocator = shared.allocator();
    let worker = shared.register_collector_worker();

    SharedWorkerHeap {
        heap: shared,
        allocator,
        worker,
    }
}
