use std::sync::Arc;

use tspp_heap::{
    AllocationCache, DEFAULT_MEMORY_MAP_SIZE_BYTES, Heap, HeapLimits, HeapOptions, SharedHeap,
    SharedHeapLimits, SharedHeapOptions, SharedMarkWorker,
};
use tspp_memory::MemoryMap;

/// One shared heap with one worker-local allocation cache.
pub(crate) struct WorkerHeap {
    /// The shared heap under test.
    pub(crate) heap: SharedHeap,
    /// The worker-local allocation cache.
    pub(crate) cache: AllocationCache,
    /// The shared mark worker used by this worker heap.
    pub(crate) worker: SharedMarkWorker,
}

/// Build one local heap suitable for allocation benchmarks.
pub(crate) fn local_heap() -> Heap {
    let options = HeapOptions::local();
    let memory = local_memory();

    Heap::new(memory, HeapLimits::default(), options).expect("heap should build")
}

/// Reserve one memory map for a local benchmark heap.
pub(crate) fn local_memory() -> Arc<MemoryMap> {
    let options = HeapOptions::local();

    Arc::new(
        MemoryMap::reserve(DEFAULT_MEMORY_MAP_SIZE_BYTES, options.page_size_bytes)
            .expect("memory map should reserve"),
    )
}

/// Build one shared heap without registering a worker.
pub(crate) fn shared_heap() -> SharedHeap {
    let options = SharedHeapOptions::default();
    let memory = Arc::new(
        MemoryMap::reserve(DEFAULT_MEMORY_MAP_SIZE_BYTES, options.page_size_bytes)
            .expect("memory map should reserve"),
    );

    SharedHeap::new(memory, SharedHeapLimits::default(), options).expect("shared heap should build")
}

/// Build one shared heap and worker-local allocation cache.
pub(crate) fn shared_worker_heap() -> WorkerHeap {
    let shared = shared_heap();
    let cache = shared.allocation_cache();
    let worker = shared.register_mark_worker();

    WorkerHeap {
        heap: shared,
        cache,
        worker,
    }
}
