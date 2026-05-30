use destack_engine as engine;
use destack_heap::{AllocationCache, GcWorker, Heap, SharedHeap};

use crate::diagnostic::Error;

use super::Machine;

/// Active VM execution state.
pub(crate) struct Activation<'run> {
    /// The durable machine state being executed.
    pub(crate) machine: &'run mut Machine,
    /// Runtime static memory for this execution.
    pub(crate) statics: &'run mut engine::StaticSpace,
    /// Worker local heap for this execution.
    pub(crate) heap: &'run mut Heap,
    /// Runtime shared heap for this execution.
    pub(crate) shared: &'run SharedHeap,
    /// Shared collector worker for this execution.
    pub(crate) shared_gc: &'run GcWorker,
    /// Worker shared allocation cache for this execution.
    pub(crate) shared_cache: &'run mut AllocationCache,
    /// Index of the active frame in the machine stack.
    pub(crate) active_frame_index: usize,
    /// Native address of the active frame bytes.
    pub(crate) active_frame_base: usize,
    /// Active frame layout.
    pub(crate) active_frame_layout: engine::FrameLayoutId,
}

impl<'run> Activation<'run> {
    /// Bind durable machine state to runtime memory for execution.
    pub(crate) fn new(
        machine: &'run mut Machine,
        statics: &'run mut engine::StaticSpace,
        heap: &'run mut Heap,
        shared: &'run SharedHeap,
        shared_gc: &'run GcWorker,
        shared_cache: &'run mut AllocationCache,
    ) -> Self {
        Self {
            machine,
            statics,
            heap,
            shared,
            shared_gc,
            shared_cache,
            active_frame_index: 0,
            active_frame_base: 0,
            active_frame_layout: engine::FrameLayoutId(0),
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

        self.active_frame_index = frame_index;
        self.active_frame_base = frame_base;
        self.active_frame_layout = frame_layout;

        Ok(())
    }
}
