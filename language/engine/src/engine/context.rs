use std::ffi::c_void;
use std::ptr::NonNull;

use destack_heap as heap;

use crate::StaticSpace;

/// Opaque runtime context for engine-specific helpers.
pub type RuntimeContext = c_void;

/// Memory available to one engine operation.
pub struct MemoryContext<'a> {
    /// Worker heap.
    pub heap: &'a mut heap::Heap,
    /// Runtime heap.
    pub shared_heap: &'a heap::SharedHeap,
    /// Worker-local shared heap allocator.
    pub shared_allocator: &'a mut heap::SharedAllocator,
    /// Runtime heap collector worker.
    pub shared_gc_worker: &'a heap::SharedGcWorker,
    /// Worker static memory.
    pub worker_static: &'a mut StaticSpace,
    /// Runtime static memory.
    pub runtime_static: &'a StaticSpace,
}

impl std::fmt::Debug for MemoryContext<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MemoryContext")
            .field("heap", &"<heap>")
            .field("shared_heap", &"<shared heap>")
            .field("shared_allocator", &"<shared allocator>")
            .field("shared_gc_worker", &"<shared gc worker>")
            .field("worker_static", &self.worker_static.len())
            .field("runtime_static", &self.runtime_static.byte_len())
            .finish()
    }
}

/// Execution context for one engine call.
pub struct CallContext<'a> {
    /// Runtime-owned opaque context for helper calls.
    pub runtime: NonNull<RuntimeContext>,
    /// Memory available to this call.
    pub memory: MemoryContext<'a>,
}

impl std::fmt::Debug for CallContext<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CallContext")
            .field("runtime", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
