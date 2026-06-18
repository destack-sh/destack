use std::ffi::c_void;
use std::ptr::NonNull;

use destack_heap as heap;

use crate::StaticSpace;

/// Opaque host call state for execution helper calls.
pub type HostCall = c_void;

/// Memory available to one execution operation.
pub struct ExecutionMemory<'a> {
    /// Worker heap.
    pub heap: &'a mut heap::Heap,
    /// Runtime heap.
    pub shared_heap: &'a heap::SharedHeap,
    /// Worker-local shared allocation cache.
    pub shared_cache: &'a mut heap::AllocationCache,
    /// Runtime heap collector worker.
    pub shared_gc_worker: &'a heap::GcWorker,
    /// Local static memory.
    pub local_static: &'a mut StaticSpace,
    /// Shared static memory.
    pub shared_static: &'a mut StaticSpace,
    /// Program constant memory.
    pub constant_space: &'a StaticSpace,
}

impl std::fmt::Debug for ExecutionMemory<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExecutionMemory")
            .field("heap", &"<heap>")
            .field("shared_heap", &"<shared heap>")
            .field("shared_cache", &"<shared allocation cache>")
            .field("shared_gc_worker", &"<shared gc worker>")
            .field("local_static", &self.local_static.len())
            .field("shared_static", &self.shared_static.len())
            .field("constant_space", &self.constant_space.byte_len())
            .finish()
    }
}

/// One execution call.
pub struct ExecutionCall<'a> {
    /// Runtime-owned host call state.
    pub host: NonNull<HostCall>,
    /// Memory available to this call.
    pub memory: ExecutionMemory<'a>,
}

impl std::fmt::Debug for ExecutionCall<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExecutionCall")
            .field("host", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
