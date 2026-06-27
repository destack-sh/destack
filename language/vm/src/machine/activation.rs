use destack_heap::{AllocationCache, Heap, SharedHeap, SharedMarkWorker};
use destack_program as program;

use crate::diagnostic::Error;

use super::Machine;

/// Active VM execution state.
pub(crate) struct Activation<'run> {
    /// The durable machine state being executed.
    pub(crate) machine: &'run mut Machine,
    /// Local static memory for this execution.
    pub(crate) local_static: &'run mut program::StaticSpace,
    /// Shared static memory for this execution.
    pub(crate) shared_static: &'run mut program::StaticSpace,
    /// Worker local heap for this execution.
    pub(crate) heap: &'run mut Heap,
    /// Runtime shared heap for this execution.
    pub(crate) shared: &'run SharedHeap,
    /// Shared mark worker for this execution.
    pub(crate) shared_mark_worker: &'run SharedMarkWorker,
    /// Worker shared allocation cache for this execution.
    pub(crate) shared_cache: &'run mut AllocationCache,
    /// Index of the active frame in the machine stack.
    pub(crate) frame_index: usize,
    /// Native address of the active frame bytes.
    pub(crate) frame_base: usize,
    /// Active frame layout.
    pub(crate) frame_layout: program::FrameLayoutId,
}

impl<'run> Activation<'run> {
    /// Bind durable machine state to runtime memory for execution.
    pub(crate) fn new(
        machine: &'run mut Machine,
        local_static: &'run mut program::StaticSpace,
        shared_static: &'run mut program::StaticSpace,
        heap: &'run mut Heap,
        shared: &'run SharedHeap,
        shared_mark_worker: &'run SharedMarkWorker,
        shared_cache: &'run mut AllocationCache,
    ) -> Self {
        Self {
            machine,
            local_static,
            shared_static,
            heap,
            shared,
            shared_mark_worker,
            shared_cache,
            frame_index: 0,
            frame_base: 0,
            frame_layout: 0.into(),
        }
    }

    /// Bind one active frame for lowered instruction execution.
    pub(crate) fn bind_frame(&mut self, frame_index: usize) -> Result<(), Error> {
        let frame = self
            .machine
            .frames
            .get(frame_index)
            .ok_or(Error::invalid_instruction())?;
        let frame_base = frame.base_address();
        let frame_layout = frame.frame_layout();
        let _frame_layout = self
            .machine
            .program
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::invalid_instruction())?;

        self.frame_index = frame_index;
        self.frame_base = frame_base;
        self.frame_layout = frame_layout;

        Ok(())
    }
}
