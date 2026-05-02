use destack_heap as heap;

use crate::StaticSpace;

/// Memory available to one engine call.
pub struct Context<'a> {
    /// Worker heap.
    pub heap: &'a mut heap::Heap,
    /// Runtime heap.
    pub shared_heap: &'a heap::SharedHeap,
    /// Runtime heap collector worker.
    pub shared_gc: &'a heap::SharedGcWorker,
    /// Worker static memory.
    pub worker_static: &'a mut StaticSpace,
    /// Runtime static memory.
    pub runtime_static: &'a StaticSpace,
}

impl std::fmt::Debug for Context<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Context")
            .field("heap", &"<heap>")
            .field("shared_heap", &"<shared heap>")
            .field("shared_gc", &"<shared gc worker>")
            .field("worker_static", &self.worker_static.len())
            .field("runtime_static", &self.runtime_static.byte_len())
            .finish()
    }
}
