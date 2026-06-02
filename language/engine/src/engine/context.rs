use std::ffi::c_void;
use std::ptr::NonNull;

use destack_heap as heap;

use crate::StaticSpace;

/// Opaque host call state for engine helper calls.
pub type HostCall = c_void;

/// Memory available to one engine operation.
pub struct EngineMemory<'a> {
    /// Worker heap.
    pub heap: &'a mut heap::Heap,
    /// Runtime heap.
    pub shared_heap: &'a heap::SharedHeap,
    /// Worker-local shared allocation cache.
    pub shared_cache: &'a mut heap::AllocationCache,
    /// Runtime heap collector worker.
    pub shared_gc_worker: &'a heap::GcWorker,
    /// Worker static memory.
    pub worker_static: &'a mut StaticSpace,
    /// Runtime static memory.
    pub runtime_static: &'a StaticSpace,
}

impl std::fmt::Debug for EngineMemory<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EngineMemory")
            .field("heap", &"<heap>")
            .field("shared_heap", &"<shared heap>")
            .field("shared_cache", &"<shared allocation cache>")
            .field("shared_gc_worker", &"<shared gc worker>")
            .field("worker_static", &self.worker_static.len())
            .field("runtime_static", &self.runtime_static.byte_len())
            .finish()
    }
}

/// Engine execution call.
pub struct EngineCall<'a> {
    /// Runtime-owned host call state.
    pub host: NonNull<HostCall>,
    /// Memory available to this call.
    pub memory: EngineMemory<'a>,
}

impl std::fmt::Debug for EngineCall<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EngineCall")
            .field("host", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
