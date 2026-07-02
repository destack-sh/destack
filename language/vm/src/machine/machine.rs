use destack_serde::Reflect;
use std::fmt;
use std::sync::Arc;

use destack_core::{Capture, CaptureMode, SnapshotCodec};
use destack_heap::{
    AllocationCache, AllocationShape, Heap, HeapReference, HeapResult, RootSlot, SharedHeap,
    SharedMarkWorker, TraceView,
};
use destack_program as program;
use destack_program::{FrameLayout, GlobalLocation, Program};
use program::{StaticImage, StaticSpace};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult, StackTraceFrame};
use crate::options::{LimitOptions, MachineOptions};
use crate::{Cell, Result as VmResult};

use super::{Continuation, Frame, FrameImage, Stack, StackImage};

/// Coroutine-capable machine outcome.
pub type Outcome = program::Outcome<Continuation, program::Value>;

/// Durable VM execution state.
pub struct Machine {
    /// Immutable program shared by this machine.
    pub(crate) program: Arc<Program>,
    /// Configuration options for this machine.
    pub(crate) options: MachineOptions,

    /// Explicit frame stack used for execution and root walking.
    pub(crate) frames: Vec<Frame>,
    /// Page-backed byte stack for frame data.
    pub(crate) stack: Stack,
    /// Last fallible allocation failure observed by this machine.
    pub(crate) last_allocation_failure: Option<Error>,
}

/// Immutable machine image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct MachineImage {
    /// The machine configuration options.
    pub options: MachineOptions,
    /// The captured stack bytes.
    pub stack: StackImage,
    /// The captured frame stack.
    pub frames: Vec<FrameImage>,
}

impl fmt::Debug for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Machine")
            .field("program", &self.program)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl Machine {
    /// Create a new machine for one durable program.
    pub fn new(program: Arc<Program>, options: MachineOptions) -> RuntimeResult<Self> {
        Self::require_program_compatibility(program.as_ref(), &options)?;
        let stack = Stack::new(options.limits.stack_bytes)?;

        Ok(Self {
            program,
            options,
            frames: Vec::new(),
            stack,
            last_allocation_failure: None,
        })
    }

    /// Return the immutable program handle.
    #[inline]
    pub fn program_handle(&self) -> Arc<Program> {
        self.program.clone()
    }

    /// Return the immutable program.
    #[inline]
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Return compact program trace rows.
    #[inline]
    pub fn trace_view(&self) -> TraceView<'_> {
        self.program.trace_view()
    }

    /// Return immutable program constants.
    #[inline]
    pub fn constants(&self) -> &StaticImage {
        self.program.constants()
    }

    /// Return initial shared static storage.
    #[inline]
    pub fn shared_statics(&self) -> &StaticImage {
        self.program.shared_statics()
    }

    /// Restore one machine from one shared immutable image.
    pub fn from_image(program: Arc<Program>, image: Arc<MachineImage>) -> RuntimeResult<Self> {
        Self::require_program_compatibility(program.as_ref(), &image.options)?;
        let (stack, frames) =
            Self::restore_stack_and_frames(&program, &image.stack, &image.frames, &image.options)?;

        Ok(Self {
            program,
            options: image.options.clone(),
            frames,
            stack,
            last_allocation_failure: None,
        })
    }

    /// Initialize heap-shaped program tables and static bytes.
    pub fn initialize(
        &mut self,
        heap: &Heap,
        shared: &SharedHeap,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
    ) -> RuntimeResult<()> {
        // reject mismatched program and heap allocation shapes
        if heap.options() != self.program.heap_options()
            || shared.options() != self.program.shared_heap_options()
        {
            return Err(self.runtime_error(Error::invalid_program(
                "program heap options do not match runtime heap options",
            )));
        }

        // initialize static data
        self.program.initialize_statics(local_static, shared_static);

        Ok(())
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(&self, name: &str) -> Result<program::FunctionId, RuntimeError> {
        let func_id = self
            .program
            .function_id_by_name(name)
            .ok_or_else(|| self.runtime_error(Error::import_not_found(name)))?;

        Ok(func_id)
    }

    /// Resolve one runtime entry name into an execution entry handle.
    pub fn entry_by_name(&self, name: &str) -> Result<program::EntryPoint, RuntimeError> {
        let function = self.function_id_by_name(name)?;

        Ok(program::EntryPoint::from(function))
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        func_id: program::FunctionId,
        arguments: &[program::Value],
    ) -> RuntimeResult<program::Value> {
        let arguments = arguments.iter().map(Cell::from).collect::<Vec<_>>();

        self.run_function_cells(
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            func_id,
            &arguments,
        )
    }

    /// Run a function by id with VM cells and return its output.
    pub(crate) fn run_function_cells(
        &mut self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        func_id: program::FunctionId,
        arguments: &[Cell],
    ) -> RuntimeResult<program::Value> {
        let program = Arc::clone(&self.program);
        let limits = self.options.limits;

        self.execute_function_cells(
            program.as_ref(),
            limits,
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            func_id,
            arguments,
        )
    }

    /// Run a function by id and allow yielding.
    pub fn run_function_yielding(
        &mut self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        func_id: program::FunctionId,
        arguments: &[program::Value],
    ) -> RuntimeResult<Outcome> {
        let arguments = arguments.iter().map(Cell::from).collect::<Vec<_>>();

        self.run_function_cells_yielding(
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            func_id,
            &arguments,
        )
    }

    /// Run a function by id with VM cells and allow yielding.
    pub(crate) fn run_function_cells_yielding(
        &mut self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        func_id: program::FunctionId,
        arguments: &[Cell],
    ) -> RuntimeResult<Outcome> {
        let program = Arc::clone(&self.program);
        let limits = self.options.limits;

        self.execute_function_cells_yielding(
            program.as_ref(),
            limits,
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            func_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    pub fn resume(
        &mut self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        continuation: Continuation,
        resume_value: program::Value,
    ) -> RuntimeResult<Outcome> {
        let program = Arc::clone(&self.program);
        let limits = self.options.limits;

        self.execute_resume(
            program.as_ref(),
            limits,
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            continuation,
            resume_value,
        )
    }

    /// Continue a materialized continuation without a received value.
    pub fn continue_continuation(
        &mut self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_mark_worker: &SharedMarkWorker,
        continuation: Continuation,
    ) -> RuntimeResult<Outcome> {
        let program = Arc::clone(&self.program);
        let limits = self.options.limits;

        self.execute_continue(
            program.as_ref(),
            limits,
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            continuation,
        )
    }

    /// Visit mutable heap root slots from live state and optional continuations.
    pub fn visit_root_slots(
        &mut self,
        local_static: &mut StaticSpace,
        continuations: &mut [Continuation],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        let program = self.program.clone();

        self.visit_root_slots_with_program(program.as_ref(), local_static, continuations, visit)
    }

    /// Visit mutable heap root slots from one static space.
    pub fn visit_static_root_slots(
        &mut self,
        location: GlobalLocation,
        static_space: &mut StaticSpace,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        self.program
            .visit_static_root_slots(location, static_space, visit)
            .map_err(|error| self.runtime_error(error.into()))
    }

    /// Capture one immutable VM image.
    pub fn image(&self) -> RuntimeResult<MachineImage> {
        let stack = self.stack.image()?;
        let frames = self
            .frames
            .iter()
            .map(|frame| frame.image(&self.program))
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(MachineImage {
            options: self.options.clone(),
            stack,
            frames,
        })
    }

    /// Fork this machine for one child branch.
    pub fn fork(&self) -> RuntimeResult<Self> {
        let (stack, frames) = self.fork_stack_and_frames()?;

        Ok(Self {
            program: self.program.clone(),
            options: self.options.clone(),
            frames,
            stack,
            last_allocation_failure: self.last_allocation_failure.clone(),
        })
    }

    /// Restore this machine from one immutable VM image.
    pub fn restore_image(&mut self, image: &MachineImage) -> RuntimeResult<()> {
        self.options = image.options.clone();

        // rebuild stack and frames over the restored program
        let (stack, frames) = Self::restore_stack_and_frames(
            &self.program,
            &image.stack,
            &image.frames,
            &self.options,
        )?;
        self.stack = stack;
        self.frames = frames;
        self.last_allocation_failure = None;

        Ok(())
    }

    /// Create a runtime error with current call stack.
    pub(crate) fn runtime_error(&self, error: Error) -> RuntimeError {
        self.runtime_error_with_program(&self.program, error)
    }

    /// Prepare the stack arena for one top-level run.
    pub(crate) fn reset_stack(&mut self, limits: LimitOptions) -> RuntimeResult<()> {
        self.frames.clear();
        self.last_allocation_failure = None;
        self.stack.reset(limits.stack_bytes)?;

        Ok(())
    }

    /// Allocate one frame byte record in the stack arena.
    pub(crate) fn allocate_frame(&mut self, layout: &FrameLayout) -> RuntimeResult<(usize, usize)> {
        let base = self
            .stack
            .allocate_zeroed(layout.byte_len() as usize, Cell::BYTE_LEN)?;
        let frame_base = self.stack.address(base, layout.byte_len() as usize)?;

        Ok((base, frame_base))
    }

    /// Release stack bytes above one frame base.
    pub(crate) fn truncate_stack(&mut self, stack_offset: usize) {
        debug_assert!(stack_offset <= self.stack.len());
        self.stack.truncate(stack_offset);
    }

    /// Fork stack and frames for one child machine.
    pub(crate) fn fork_stack_and_frames(&self) -> RuntimeResult<(Stack, Vec<Frame>)> {
        let stack = self.stack.fork()?;
        let mut frames = Vec::with_capacity(self.frames.len());

        // clone frames over the forked stack bytes
        for frame in &self.frames {
            let base = stack
                .address(frame.stack_offset, frame.byte_len())
                .map_err(|_| RuntimeError::new(Error::invalid_continuation()))?;
            frames.push(frame.fork(base));
        }

        Ok((stack, frames))
    }

    /// Restore stack and frames from one immutable image.
    pub(crate) fn restore_stack_and_frames(
        program: &Program,
        stack_image: &StackImage,
        frame_images: &[FrameImage],
        options: &MachineOptions,
    ) -> RuntimeResult<(Stack, Vec<Frame>)> {
        let stack = Stack::from_image(stack_image, options.limits.stack_bytes)?;
        let mut frames = Vec::with_capacity(frame_images.len());

        // restore frame tables over stack image byte ranges
        for frame_image in frame_images {
            let frame_base = stack.address(frame_image.stack_offset, frame_image.byte_len())?;
            let frame =
                Frame::from_image(frame_image, program, frame_image.stack_offset, frame_base)?;

            frames.push(frame);
        }

        Ok((stack, frames))
    }

    /// Create a runtime error with current call stack.
    #[cold]
    pub(crate) fn runtime_error_with_program(
        &self,
        program: &Program,
        error: Error,
    ) -> RuntimeError {
        match self.call_stack(program) {
            Ok(stack) => RuntimeError::new(error).with_call_stack(stack),
            Err(stack_error) => RuntimeError::new(Error::internal(format!(
                "failed to build call stack for {error:?}: {stack_error:?}"
            ))),
        }
    }

    /// Return the current call stack for error reporting.
    fn call_stack(&self, program: &Program) -> VmResult<Vec<StackTraceFrame>> {
        self.frames
            .iter()
            .map(|frame| {
                let function = frame.function();
                let function_ref = program
                    .vm_function_by_id(function)
                    .ok_or_else(|| Error::undefined_function(function))?;
                let block = frame.block;
                function_ref
                    .blocks
                    .get(block as usize)
                    .ok_or_else(Error::invalid_instruction)?;
                let function_name = program
                    .function(function)
                    .and_then(|function| program.string(function.name).map(str::to_owned));

                Ok(StackTraceFrame {
                    function,
                    block,
                    function_name,
                })
            })
            .collect()
    }

    /// Visit mutable heap root slots from active frames and suspended continuations.
    pub(crate) fn visit_root_slots_with_program(
        &mut self,
        program: &Program,
        local_static: &mut StaticSpace,
        continuations: &mut [Continuation],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        // active frames
        for frame in &mut self.frames {
            let result = frame.visit_root_slots(program, visit);
            if let Err(error) = result {
                return Err(self.runtime_error_with_program(program, error));
            }
        }

        // suspended continuations
        for continuation in continuations {
            Self::visit_continuation_slots(program, continuation, visit)
                .map_err(|error| self.runtime_error_with_program(program, error))?;
        }

        program
            .visit_static_root_slots(GlobalLocation::LocalStatic, local_static, visit)
            .map_err(|error| self.runtime_error_with_program(program, error.into()))?;

        Ok(())
    }

    /// Require machine options compatible with the program header.
    fn require_program_compatibility(
        program: &Program,
        options: &MachineOptions,
    ) -> RuntimeResult<()> {
        let pointer_bytes = program.pointer_bytes();
        let host_pointer_bytes = HeapReference::BYTE_LEN as u8;

        // host execution only supports native-width pointers
        if pointer_bytes != host_pointer_bytes {
            return Err(RuntimeError::new(Error::incompatible_pointer_width(
                pointer_bytes,
                host_pointer_bytes,
            )));
        }

        if program.heap_options() != &options.heap {
            return Err(RuntimeError::new(Error::invalid_program(
                "program local heap options do not match machine options",
            )));
        }

        if program.shared_heap_options() != &options.shared_heap {
            return Err(RuntimeError::new(Error::invalid_program(
                "program shared heap options do not match machine options",
            )));
        }

        Ok(())
    }

    /// Borrow the canonical runtime layout table.
    pub fn layout_table(&self) -> &program::LayoutTable {
        self.program.layouts()
    }

    /// Return the canonical layout id for one program type.
    pub fn layout_id_for_type(&self, ty: program::TypeId) -> Option<program::LayoutId> {
        self.program.layout_id_for_type(ty)
    }

    /// Return the canonical layout for one program type.
    #[cfg(test)]
    pub(crate) fn layout(&self, ty: program::TypeId) -> Option<&program::Layout> {
        self.program.layout(ty)
    }

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: program::LayoutId) -> VmResult<AllocationShape> {
        Ok(self.program.allocation_shape(layout_id)?)
    }
}

impl Capture for Machine {
    type Image = MachineImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = &'a mut Heap;

    /// Capture one machine image for the given mode.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Machine::image(self).map_err(Box::<RuntimeError>::from)
    }

    /// Restore one machine image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _heap: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        Machine::restore_image(self, image).map_err(Box::<RuntimeError>::from)
    }
}

impl SnapshotCodec for Machine {
    type Snapshot = MachineImage;

    /// Encode one machine image as one snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(image.clone())
    }

    /// Decode one machine snapshot back into one image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(snapshot.clone())
    }
}
