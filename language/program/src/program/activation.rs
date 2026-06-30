use std::ffi::c_void;
use std::fmt;
use std::ptr::NonNull;

use destack_heap::{AllocationCache, Heap, SharedHeap, SharedMarkWorker};

use crate::{StaticImage, StaticSpace};

/// One call from the runtime into a program machine.
pub struct ProgramActivation<'a> {
    /// Runtime-owned call state.
    pub state: NonNull<c_void>,
    /// Memory available to this call.
    pub storage: ProgramStorage<'a>,
}

impl fmt::Debug for ProgramActivation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProgramActivation")
            .field("state", &true)
            .field("storage", &self.storage)
            .finish()
    }
}

/// Memory available to one runtime call.
pub struct ProgramStorage<'a> {
    /// Worker heap.
    pub heap: &'a mut Heap,
    /// Runtime heap.
    pub shared_heap: &'a SharedHeap,
    /// Worker-local shared allocation cache.
    pub shared_cache: &'a mut AllocationCache,
    /// Shared heap mark worker.
    pub shared_mark_worker: &'a SharedMarkWorker,
    /// Local static memory.
    pub local_static: &'a mut StaticSpace,
    /// Shared static memory.
    pub shared_static: &'a mut StaticSpace,
    /// Program constant memory.
    pub constant_space: &'a StaticImage,
}

impl fmt::Debug for ProgramStorage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProgramStorage")
            .field("heap", &"<heap>")
            .field("shared_heap", &"<shared heap>")
            .field("shared_cache", &"<shared allocation cache>")
            .field("shared_mark_worker", &"<shared mark worker>")
            .field("local_static", &self.local_static.byte_len())
            .field("shared_static", &self.shared_static.byte_len())
            .field("constant_space", &"<constant image>")
            .finish()
    }
}
