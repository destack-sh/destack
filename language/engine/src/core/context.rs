use destack_heap as heap;

use crate::StaticSpace;

/// Heap and static memory available to one backend execution call.
pub struct Context<'a> {
    /// Worker-local heap.
    pub heap: &'a mut heap::Heap,
    /// Runtime-shared heap.
    pub shared: &'a heap::SharedHeap,
    /// Shared collector worker handle for this execution worker.
    pub shared_gc: &'a heap::SharedGcWorker,
    /// Worker-owned static bytes.
    pub worker_static: &'a mut StaticSpace,
    /// Runtime-owned static bytes.
    pub runtime_static: &'a StaticSpace,
}

impl std::fmt::Debug for Context<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Context")
            .field("heap", &"<heap>")
            .field("shared", &"<shared heap>")
            .field("shared_gc", &"<shared gc worker>")
            .field("worker_static", &self.worker_static.len())
            .field("runtime_static", &self.runtime_static.byte_len())
            .finish()
    }
}
