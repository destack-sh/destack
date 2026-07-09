use destack_heap::{AllocationCache, Heap, SharedHeap, SharedMarkWorker};
use destack_program as program;
use program::Program;
use program::vm::FunctionCode;

use crate::diagnostic::Error;

use super::Machine;

/// Active VM execution state.
pub(crate) struct Activation<'run> {
    /// Immutable program being executed.
    pub(crate) program: &'run Program,
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
    /// Active executable stop points for this execution.
    pub(crate) stop_points: Option<&'run program::StopSet>,
    /// Active executable watchpoints for this execution.
    pub(crate) watch_points: Option<&'run program::WatchSet>,
    /// Mutable runtime profile for this execution.
    pub(crate) profile: Option<&'run mut program::Profile>,
    /// Stop skipped once while continuing a retained stop.
    pub(crate) resume_skip: Option<program::ResumeSkip>,
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
        program: &'run Program,
        machine: &'run mut Machine,
        local_static: &'run mut program::StaticSpace,
        shared_static: &'run mut program::StaticSpace,
        heap: &'run mut Heap,
        shared: &'run SharedHeap,
        shared_mark_worker: &'run SharedMarkWorker,
        shared_cache: &'run mut AllocationCache,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> Self {
        Self {
            program,
            machine,
            local_static,
            shared_static,
            heap,
            shared,
            shared_mark_worker,
            shared_cache,
            stop_points,
            watch_points,
            profile,
            resume_skip,
            frame_index: 0,
            frame_base: 0,
            frame_layout: 0.into(),
        }
    }

    /// Return whether this activation carries executable stop points.
    pub(crate) fn has_stop_points(&self) -> bool {
        self.stop_points.is_some_and(|stops| !stops.is_empty())
    }

    /// Return whether this activation carries executable watchpoints.
    pub(crate) fn has_watch_points(&self) -> bool {
        self.watch_points.is_some_and(|watches| !watches.is_empty())
    }

    /// Return whether this activation records runtime profile data.
    pub(crate) fn has_profile(&self) -> bool {
        self.profile.is_some()
    }

    /// Bind one active frame for lowered instruction execution.
    pub(crate) fn bind_frame(&mut self, frame_index: usize) -> Result<(), Error> {
        // resolve the durable frame state
        let frame = self
            .machine
            .frames
            .get(frame_index)
            .ok_or(Error::invalid_instruction())?;
        let frame_base = frame.base_address();
        let frame_layout = frame.frame_layout();
        let _frame_layout = self
            .program
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::invalid_instruction())?;

        self.frame_index = frame_index;
        self.frame_base = frame_base;
        self.frame_layout = frame_layout;

        Ok(())
    }

    /// Return the executable point at one lowered instruction coordinate.
    pub(super) fn program_point_at(
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
    ) -> Result<program::ProgramPoint, Error> {
        let operation = function
            .operation_at(block, pc as u32)
            .ok_or(Error::invalid_instruction())?;

        Ok(program::ProgramPoint::new(
            function.function.function,
            operation,
        ))
    }
}
