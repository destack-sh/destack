use std::sync::Arc;

use destack_heap::{
    Allocator, Heap, HeapLimits, HeapOptions, SharedAllocationCache, SharedGcWorker, SharedHeap,
    SharedHeapLimits, SharedHeapOptions,
};

/// One shared heap with one worker-local allocator.
pub(crate) struct SharedWorkerHeap {
    /// The shared heap under test.
    pub(crate) heap: SharedHeap,
    /// The worker-local allocator cache.
    pub(crate) allocator: SharedAllocationCache,
    /// The shared collector worker used by this worker heap.
    pub(crate) worker: SharedGcWorker,
}

/// Build one local heap suitable for allocation benchmarks.
pub(crate) fn local_heap() -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("heap should build")
}

/// Build one shared heap without registering a worker.
pub(crate) fn shared_heap() -> SharedHeap {
    let options = SharedHeapOptions::default();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("allocator should build"),
    );

    SharedHeap::with_allocator_limits_and_options(allocator, SharedHeapLimits::default(), options)
        .expect("shared heap should build")
}

/// Build one shared heap and worker-local allocator.
pub(crate) fn shared_worker_heap() -> SharedWorkerHeap {
    let shared = shared_heap();
    let allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    SharedWorkerHeap {
        heap: shared,
        allocator,
        worker,
    }
}
