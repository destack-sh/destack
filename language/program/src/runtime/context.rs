use std::ffi::c_void;
use std::ptr::NonNull;

use destack_heap as heap;

use crate::StaticSpace;

/// Opaque runtime-owned call state.
pub type RuntimeState = c_void;

/// Memory available to one runtime call.
pub struct RuntimeMemory<'a> {
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

impl std::fmt::Debug for RuntimeMemory<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeMemory")
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

/// One call from the runtime into a program machine.
pub struct RuntimeCall<'a> {
    /// Runtime-owned call state.
    pub state: NonNull<RuntimeState>,
    /// Memory available to this call.
    pub memory: RuntimeMemory<'a>,
}

impl std::fmt::Debug for RuntimeCall<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeCall")
            .field("state", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
