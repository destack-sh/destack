use std::fmt;
use std::sync::Arc;

use destack_core::{Capture, CaptureMode, SnapshotCodec, StringPool};
use destack_engine as engine;
use destack_heap::{
    AllocationCache, AllocationShape, GcWorker, Heap, HeapReference, HeapResult, RootSlot,
    SharedHeap,
};
use destack_mir as mir;
use engine::StaticSpace;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult, StackTraceFrame};
use crate::options::{LimitOptions, MachineOptions};
#[cfg(test)]
use crate::program::Layout;
use crate::program::Program;
use crate::{Result as VmResult, Word};

use super::{Continuation, ContinuationImage, Frame, FrameImage, Stack, StackImage};

/// Coroutine-capable machine outcome.
pub type Outcome = engine::Outcome<Continuation, engine::Value>;

/// Durable VM execution state.
pub struct Machine {
    /// Unique id used to validate continuation ownership.
    pub(crate) id: engine::EngineId,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineImage {
    /// The MIR tree used to rebuild the machine program.
    pub tree: mir::Tree,
    /// The string pool used to rebuild the machine program.
    pub strings: StringPool,
    /// The machine configuration options.
    pub options: MachineOptions,
    /// The machine id captured in this image.
    pub machine_id: engine::EngineId,
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
    /// Return the canonical program trace table.
    #[inline]
    pub fn trace_table(&self) -> Arc<mir::TraceTable> {
        self.program.trace_table_handle()
    }

    /// Restore one machine from one shared immutable image.
    pub fn from_image(image: Arc<MachineImage>) -> RuntimeResult<Self> {
        let program = Arc::new(Program::with_heap_options(
            image.tree.clone(),
            image.strings.clone(),
            image.options.heap.clone(),
            image.options.shared_heap.clone(),
        )?);
        Self::require_host_pointer_width(program.as_ref())?;
        let (stack, frames) =
            Self::restore_stack_and_frames(&program, &image.stack, &image.frames, &image.options)?;

        Ok(Self {
            id: image.machine_id,
            program,
            options: image.options.clone(),
            frames,
            stack,
            last_allocation_failure: None,
        })
    }

    /// Build a new machine with custom options.
    pub fn build_with_options(
        machine_id: engine::EngineId,
        tree: mir::Tree,
        strings: StringPool,
        options: MachineOptions,
    ) -> RuntimeResult<Self> {
        let program = Arc::new(Program::with_heap_options(
            tree,
            strings,
            options.heap.clone(),
            options.shared_heap.clone(),
        )?);
        Self::require_host_pointer_width(program.as_ref())?;

        let stack = Stack::new(options.limits.stack_bytes)?;

        Ok(Self {
            id: machine_id,
            program,
            options,
            frames: Vec::new(),
            stack,
            last_allocation_failure: None,
        })
    }

    /// Initialize heap-shaped program metadata and worker static bytes.
    pub fn initialize(
        &mut self,
        heap: &Heap,
        shared: &SharedHeap,
        statics: &mut StaticSpace,
    ) -> RuntimeResult<()> {
        // reject mismatched program and heap allocation shapes
        if heap.options() != &self.options.heap || shared.options() != &self.options.shared_heap {
            return Err(self.runtime_error(Error::invalid_program(
                "machine heap options do not match runtime heap options",
            )));
        }

        // initialize static data
        self.initialize_statics(statics)
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(
        &self,
        name: &str,
    ) -> Result<mir::LocalNodeId<mir::Function>, RuntimeError> {
        let func_id = self
            .program
            .function_id_by_name(name)
            .ok_or_else(|| self.runtime_error(Error::import_not_found(name)))?;

        Ok(func_id)
    }

    /// Resolve one runtime entry name into an engine entry handle.
    pub fn entry_by_name(&self, name: &str) -> Result<engine::EntryPoint, RuntimeError> {
        let function = self.function_id_by_name(name)?;

        Ok(engine::EntryPoint::new(function.id))
    }

    /// Resolve one engine entry into a MIR function id.
    pub(crate) fn function_for_entry(
        &self,
        entry: engine::EntryPoint,
    ) -> mir::LocalNodeId<mir::Function> {
        self.program.function_for_entry(entry)
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_gc: &GcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_words(
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            func_id,
            &arguments,
        )
    }

    /// Run a function by id with VM words and return its output.
    pub(crate) fn run_function_words(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_gc: &GcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<engine::Value> {
        let program = Arc::clone(&self.program);
        let limits = self.options.limits;

        self.execute_function_words(
            program.as_ref(),
            limits,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            func_id,
            arguments,
        )
    }

    /// Run a function by id and allow yielding.
    pub fn run_function_yielding(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_gc: &GcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[engine::Value],
    ) -> RuntimeResult<Outcome> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_words_yielding(
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            func_id,
            &arguments,
        )
    }

    /// Run a function by id with VM words and allow yielding.
    pub(crate) fn run_function_words_yielding(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_gc: &GcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        let program = Arc::clone(&self.program);
        let limits = self.options.limits;

        self.execute_function_words_yielding(
            program.as_ref(),
            limits,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            func_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    pub fn resume(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_cache: &mut AllocationCache,
        shared_gc: &GcWorker,
        continuation: Continuation,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome> {
        let program = Arc::clone(&self.program);
        let limits = self.options.limits;

        self.execute_resume(
            program.as_ref(),
            limits,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            continuation,
            resume_value,
        )
    }

    /// Capture one continuation as one immutable image.
    pub fn continuation_image(
        &self,
        continuation: &Continuation,
    ) -> RuntimeResult<ContinuationImage> {
        continuation.image(&self.program)
    }

    /// Restore one continuation from one immutable image.
    pub fn restore_continuation_image(
        &self,
        image: &ContinuationImage,
    ) -> RuntimeResult<Continuation> {
        if image.engine_id != self.id {
            return Err(self.runtime_error(Error::invalid_continuation()));
        }

        Continuation::from_image(image, &self.program, &self.options)
    }

    /// Visit mutable heap root slots from live state and optional continuations.
    pub fn visit_root_slots(
        &mut self,
        statics: &mut StaticSpace,
        continuations: &mut [Continuation],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        let program = self.program.clone();

        self.visit_root_slots_with_program(program.as_ref(), statics, continuations, visit)
    }

    /// Visit mutable heap root slots from one live continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        continuation
            .visit_root_slots(&self.program, visit)
            .map_err(|error| self.runtime_error(error))
    }

    /// Visit mutable heap root slots from one captured continuation image.
    pub fn visit_image_root_slots(
        &mut self,
        image: &mut ContinuationImage,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        Continuation::visit_image_root_slots(image, &self.program, visit)
            .map_err(|error| self.runtime_error(error))
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
            tree: self.program.tree.clone(),
            strings: self.program.strings.clone(),
            options: self.options.clone(),
            machine_id: self.id,
            stack,
            frames,
        })
    }

    /// Fork this machine for one child branch.
    pub fn fork(&self) -> RuntimeResult<Self> {
        let (stack, frames) = self.fork_stack_and_frames()?;

        Ok(Self {
            id: self.id,
            program: self.program.clone(),
            options: self.options.clone(),
            frames,
            stack,
            last_allocation_failure: self.last_allocation_failure.clone(),
        })
    }

    /// Restore this machine from one immutable VM image.
    pub fn restore_image(&mut self, image: &MachineImage) -> RuntimeResult<()> {
        let program = Arc::new(Program::with_heap_options(
            image.tree.clone(),
            image.strings.clone(),
            image.options.heap.clone(),
            image.options.shared_heap.clone(),
        )?);
        Self::require_host_pointer_width(program.as_ref())?;

        self.program = program;
        self.options = image.options.clone();

        self.id = image.machine_id;

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
    pub(crate) fn allocate_frame(
        &mut self,
        layout: &engine::FrameLayout,
    ) -> RuntimeResult<(usize, usize)> {
        let base = self
            .stack
            .allocate_zeroed(layout.byte_len as usize, Word::BYTE_LEN)?;
        let frame_base = self.stack.address(base, layout.byte_len as usize)?;

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
                .address(frame.stack_offset, frame.byte_len)
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

        // restore frame metadata over stack image byte ranges
        for frame_image in frame_images {
            let frame_base = stack.address(frame_image.stack_offset, frame_image.byte_len)?;
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

    /// Initialize static data from MIR globals.
    pub(crate) fn initialize_statics(&mut self, statics: &mut StaticSpace) -> RuntimeResult<()> {
        let program = self.program.clone();
        let program = program.as_ref();

        // allocate static bytes
        let mut initialized_statics = StaticSpace::allocator();

        // snapshot globals before writing static bytes
        let global_entries: Vec<_> = program
            .tree
            .iter_nodes::<mir::Global>()
            .map(|(id, global)| {
                let ty = (global.ty)
                    .ty()
                    .ok_or_else(|| Error::invalid_program("global type"))?;

                Ok((id, ty, global.is_import(), global.initializer.clone()))
            })
            .collect::<VmResult<Vec<_>>>()?;

        // populate static bytes from global initializers
        for (id, ty, is_import, initializer) in global_entries {
            // skip imported globals
            if is_import || program.contains_static(id) {
                continue;
            }

            let layout = program.layout(ty).ok_or_else(|| {
                self.runtime_error(Error::type_mismatch(
                    "compiled global layout",
                    format!("{ty:?}"),
                ))
            })?;
            let bytes = match initializer.as_ref() {
                Some(init) => program
                    .initializer_bytes(init, ty)
                    .map_err(|error| self.runtime_error(error))?,
                None => vec![0; layout.byte_len],
            };
            let was_defined = initialized_statics.define(
                program.static_id(id),
                program.value_layout_id(ty),
                layout.alignment(),
                program.tree.get(id).is_mutable(),
                &bytes,
            );
            if !was_defined {
                return Err(self.runtime_error(Error::invalid_instruction()));
            }
        }

        // store initialized static data
        *statics = initialized_statics.finish();

        Ok(())
    }

    /// Return the current call stack for error reporting.
    fn call_stack(&self, program: &Program) -> VmResult<Vec<StackTraceFrame>> {
        self.frames
            .iter()
            .map(|frame| {
                let function = frame.function();
                let block = frame.block_id(program)?;
                let function_node = program.tree.get(function);
                let function_name = program.strings.get(function_node.name).to_string();

                Ok(StackTraceFrame {
                    function,
                    block,
                    function_name: Some(function_name),
                })
            })
            .collect()
    }

    /// Visit mutable heap root slots from active frames and suspended continuations.
    pub(crate) fn visit_root_slots_with_program(
        &mut self,
        program: &Program,
        statics: &mut StaticSpace,
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
            continuation
                .visit_root_slots(program, visit)
                .map_err(|error| self.runtime_error_with_program(program, error))?;
        }

        let static_ids: Vec<_> = statics.ids().collect();

        // statics
        for id in static_ids {
            let region = statics.region(id).cloned().ok_or_else(|| {
                self.runtime_error_with_program(program, Error::invalid_instruction())
            })?;
            let bytes = statics.bytes_mut(id).ok_or_else(|| {
                self.runtime_error_with_program(program, Error::invalid_instruction())
            })?;

            program
                .visit_byte_root_slots(program.type_for_value_layout(region.layout), bytes, visit)
                .map_err(|error| self.runtime_error_with_program(program, error))?;
        }

        Ok(())
    }

    /// Require a host pointer width compatible with the program layout.
    fn require_host_pointer_width(program: &Program) -> RuntimeResult<()> {
        let pointer_bytes = program.tree.metadata.data_layout.pointer_bytes;
        let host_pointer_bytes = HeapReference::BYTE_LEN as u8;

        // host execution only supports native-width pointers
        if pointer_bytes != host_pointer_bytes {
            return Err(RuntimeError::new(Error::incompatible_pointer_width(
                pointer_bytes,
                host_pointer_bytes,
            )));
        }

        Ok(())
    }

    /// Borrow the canonical runtime layout table.
    pub fn layout_table(&self) -> &mir::LayoutTable {
        self.program.layouts()
    }

    /// Return the canonical layout id for one MIR type.
    pub fn layout_id_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<mir::LayoutId> {
        self.program.layout_id_for_type(ty)
    }

    /// Return the compiled VM layout for one MIR type.
    #[cfg(test)]
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&Layout> {
        self.program.layout(ty)
    }

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: mir::LayoutId) -> VmResult<AllocationShape<'_>> {
        self.program.allocation_shape(layout_id)
    }

    /// Borrow the program MIR tree.
    pub fn tree(&self) -> &mir::Tree {
        &self.program.tree
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
