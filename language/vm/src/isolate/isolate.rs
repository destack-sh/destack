use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use destack_core::{Capture, CaptureMode, SnapshotCodec, StringPool};
use engine::StaticSpace;
use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_mir as mir};

use super::{BindingContext, BindingFn, RootSink};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::interpreter::{Continuation, ContinuationImage, Interpreter, InterpreterImage, Outcome};
use crate::options::IsolateOptions;
use crate::program::Program;
use crate::{Result as VmResult, SharedHeap, Word};
use destack_heap::{
    AllocationShape, Heap, HeapReference, HeapResult, RootSlot, SharedAllocator, SharedGcWorker,
    SharedRawLimits,
};

/// VM isolate with static data and execution state.
pub struct Isolate {
    /// Unique id used to validate continuation ownership.
    id: engine::EngineId,
    /// Immutable program shared by this isolate.
    program: Arc<Program>,
    /// Configuration options for this isolate.
    options: IsolateOptions,
    /// Binding handlers registered for VM calls.
    bindings: HashMap<String, BindingFn>,
    /// Interpreter engine backing this isolate.
    interpreter: Interpreter,
}

/// Immutable isolate image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolateImage {
    /// The MIR tree used to rebuild the isolate program.
    pub tree: mir::Tree,
    /// The string pool used to rebuild the isolate program.
    pub strings: StringPool,
    /// The isolate configuration options.
    pub options: IsolateOptions,
    /// The isolate id captured in this image.
    pub isolate_id: engine::EngineId,
    /// The captured interpreter state.
    pub interpreter: InterpreterImage,
}

impl fmt::Debug for Isolate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Isolate")
            .field("program", &self.program)
            .field("bindings", &format!("<{} handlers>", self.bindings.len()))
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl Isolate {
    /// Create a new isolate from one shared immutable image.
    pub fn new(image: Arc<IsolateImage>) -> RuntimeResult<Self> {
        let program = Arc::new(Program::new(image.tree.clone(), image.strings.clone())?);
        Self::require_host_pointer_width(program.as_ref())?;
        let interpreter = Interpreter::from_image(&program, &image.interpreter, &image.options)?;

        Ok(Self {
            id: image.isolate_id,
            program,
            options: image.options.clone(),
            bindings: HashMap::new(),
            interpreter,
        })
    }

    /// Build a new isolate with default options.
    pub fn build(
        isolate_id: engine::EngineId,
        tree: mir::Tree,
        strings: StringPool,
    ) -> RuntimeResult<Self> {
        Self::build_with_options(isolate_id, tree, strings, IsolateOptions::default())
    }

    /// Build a new isolate with custom options.
    pub fn build_with_options(
        isolate_id: engine::EngineId,
        tree: mir::Tree,
        strings: StringPool,
        options: IsolateOptions,
    ) -> RuntimeResult<Self> {
        let program = Arc::new(Program::new(tree, strings)?);
        Self::require_host_pointer_width(program.as_ref())?;

        let interpreter = Interpreter::new(&options)?;

        Ok(Self {
            id: isolate_id,
            program,
            options,
            bindings: HashMap::new(),
            interpreter,
        })
    }

    /// Initialize heap-shaped program metadata and worker static bytes.
    pub fn initialize(
        &mut self,
        heap: &Heap,
        shared: &SharedHeap,
        statics: &mut StaticSpace,
    ) -> RuntimeResult<()> {
        // lower allocation ops for the live heap geometry
        self.program = Arc::new(Program::with_heap_options(
            self.program.tree.clone(),
            self.program.strings.clone(),
            heap.options().clone(),
            shared.options().clone(),
        )?);

        // initialize static data
        self.interpreter
            .initialize_statics(self.program.as_ref(), statics)
    }

    /// Return the isolate options.
    pub fn options(&self) -> &IsolateOptions {
        &self.options
    }

    /// Register a binding handler.
    pub fn register_binding<F>(&mut self, name: &str, handler: F)
    where
        F: for<'ctx> Fn(&mut BindingContext<'ctx>, &[Word]) -> Result<Word, Error>
            + Send
            + Sync
            + 'static,
    {
        self.bindings.insert(name.to_string(), Arc::new(handler));
    }

    // FUGU #Architecture: remove once generated ABI stops registering runtime payload schemas
    /// Accept generated runtime payload registrations.
    pub fn register_named_aggregate_type(
        &mut self,
        _name: &str,
        _field_count: usize,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Run a callback with a binding context for this isolate.
    pub fn with_binding_context<F, R>(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_raw_limits: SharedRawLimits,
        run: F,
    ) -> Result<R, Error>
    where
        F: for<'ctx> FnOnce(&mut BindingContext<'ctx>) -> Result<R, Error>,
    {
        // borrow the isolate state needed by the binding context
        let program = self.program.as_ref();
        let mut context = BindingContext::new(program, heap, shared, shared_raw_limits);
        let result = run(&mut context);

        // release pins before returning to managed code
        let release_result = context.release_pins();
        match (result, release_result) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(value),
        }
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(
        &self,
        name: &str,
    ) -> Result<mir::LocalNodeId<mir::Function>, RuntimeError> {
        let func_id = self.program.function_id_by_name(name).ok_or_else(|| {
            self.runtime_error(Error::BindingFunctionNotFound {
                name: name.to_string(),
            })
        })?;

        Ok(func_id)
    }

    /// Resolve one runtime entry name into an engine entry handle.
    pub fn entry_by_name(&self, name: &str) -> Result<engine::Entry, RuntimeError> {
        let function = self.function_id_by_name(name)?;

        Ok(engine::Entry::new(function.id))
    }

    /// Resolve one engine entry into a MIR function id.
    pub(crate) fn function_for_entry(
        &self,
        entry: engine::Entry,
    ) -> mir::LocalNodeId<mir::Function> {
        self.program.function_for_entry(entry)
    }

    /// Run a function by name and return its output.
    pub fn run_function_by_name(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        name: &str,
        arguments: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();
        let function_id = self.function_id_by_name(name)?;

        self.run_function_frame(
            statics,
            heap,
            shared,
            shared_allocator,
            shared_gc,
            function_id,
            &arguments,
        )
    }

    /// Run a function by name and allow yielding.
    pub fn run_function_by_name_yielding(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        name: &str,
        arguments: &[engine::Value],
    ) -> RuntimeResult<Outcome> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();
        let function_id = self.function_id_by_name(name)?;

        self.run_function_yielding_frame(
            statics,
            heap,
            shared,
            shared_allocator,
            shared_gc,
            function_id,
            &arguments,
        )
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[engine::Value],
    ) -> RuntimeResult<engine::Value> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_frame(
            statics,
            heap,
            shared,
            shared_allocator,
            shared_gc,
            func_id,
            &arguments,
        )
    }

    /// Run a function by id with VM frame arguments and return its output.
    pub(crate) fn run_function_frame(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<engine::Value> {
        let Self {
            id: isolate_id,
            program,
            options,
            bindings,
            interpreter,
            ..
        } = self;

        interpreter.run_function(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            bindings,
            heap,
            shared,
            shared_allocator,
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
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[engine::Value],
    ) -> RuntimeResult<Outcome> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_yielding_frame(
            statics,
            heap,
            shared,
            shared_allocator,
            shared_gc,
            func_id,
            &arguments,
        )
    }

    /// Run a function by id with VM frame arguments and allow yielding.
    pub(crate) fn run_function_yielding_frame(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        let Self {
            id: isolate_id,
            program,
            options,
            bindings,
            interpreter,
            ..
        } = self;

        interpreter.run_function_yielding(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            bindings,
            heap,
            shared,
            shared_allocator,
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
        shared_allocator: &mut SharedAllocator,
        shared_gc: &SharedGcWorker,
        continuation: Continuation,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome> {
        let Self {
            id: isolate_id,
            program,
            options,
            bindings,
            interpreter,
            ..
        } = self;

        interpreter.resume(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            bindings,
            heap,
            shared,
            shared_allocator,
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
            return Err(self.runtime_error(Error::InvalidContinuation));
        }

        Continuation::from_image(image, &self.program, &self.options)
    }

    /// Visit one complete root set from live state and optional continuations.
    pub fn visit_state_roots(
        &mut self,
        statics: &StaticSpace,
        continuations: &[Continuation],
        roots: &mut impl RootSink,
    ) -> RuntimeResult<()> {
        self.interpreter
            .visit_roots(&self.program, statics, continuations, roots)
    }

    /// Visit mutable local root slots from live state and optional continuations.
    pub fn visit_root_slots(
        &mut self,
        statics: &mut StaticSpace,
        continuations: &mut [Continuation],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        self.interpreter
            .visit_root_slots(&self.program, statics, continuations, visit)
    }

    /// Visit roots from one live continuation.
    pub fn visit_continuation_roots(
        &mut self,
        continuation: &Continuation,
        roots: &mut impl RootSink,
    ) -> RuntimeResult<()> {
        continuation
            .visit_roots(&self.program, roots)
            .map_err(|error| self.runtime_error(error))
    }

    /// Visit mutable local root slots from one live continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        continuation
            .visit_root_slots(&self.program, visit)
            .map_err(|error| self.runtime_error(error))
    }

    /// Visit roots from one captured continuation image.
    pub fn visit_image_roots(
        &mut self,
        image: &ContinuationImage,
        roots: &mut impl RootSink,
    ) -> RuntimeResult<()> {
        Continuation::visit_image_roots(image, &self.program, roots)
            .map_err(|error| self.runtime_error(error))
    }

    /// Visit mutable local root slots from one captured continuation image.
    pub fn visit_image_root_slots(
        &mut self,
        image: &mut ContinuationImage,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        Continuation::visit_image_root_slots(image, &self.program, visit)
            .map_err(|error| self.runtime_error(error))
    }

    /// Capture one immutable VM image.
    pub fn image(&self) -> RuntimeResult<IsolateImage> {
        let interpreter = self.interpreter.image();

        Ok(IsolateImage {
            tree: self.program.tree.clone(),
            strings: self.program.strings.clone(),
            options: self.options.clone(),
            isolate_id: self.id,
            interpreter,
        })
    }

    /// Fork this isolate for one child branch.
    pub fn fork(&self) -> RuntimeResult<Self> {
        Ok(Self {
            id: self.id,
            program: self.program.clone(),
            options: self.options.clone(),
            bindings: self.bindings.clone(),
            interpreter: self.interpreter.fork()?,
        })
    }

    /// Restore this isolate from one immutable VM image.
    pub fn restore_image(&mut self, image: &IsolateImage) -> RuntimeResult<()> {
        let program = Arc::new(Program::new(image.tree.clone(), image.strings.clone())?);
        Self::require_host_pointer_width(program.as_ref())?;

        self.program = program;
        self.options = image.options.clone();

        self.id = image.isolate_id;

        // rebuild interpreter state over the restored isolate
        self.interpreter =
            Interpreter::from_image(&self.program, &image.interpreter, &self.options)?;

        Ok(())
    }

    /// Create a runtime error with current call stack.
    fn runtime_error(&self, error: Error) -> RuntimeError {
        self.interpreter.runtime_error(&self.program, error)
    }

    /// Require a host pointer width compatible with the program layout.
    fn require_host_pointer_width(program: &Program) -> RuntimeResult<()> {
        let pointer_bytes = program.tree.metadata.data_layout.pointer_bytes;
        let host_pointer_bytes = HeapReference::BYTE_LEN as u8;

        // host execution only supports native-width pointers
        if pointer_bytes != host_pointer_bytes {
            return Err(RuntimeError::new(Error::IncompatiblePointerWidth {
                bytes: pointer_bytes,
                host_bytes: host_pointer_bytes,
            }));
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

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: mir::LayoutId) -> VmResult<AllocationShape<'_>> {
        self.program.allocation_shape(layout_id)
    }

    /// Borrow the program MIR tree.
    pub fn tree(&self) -> &mir::Tree {
        &self.program.tree
    }
}

impl Capture for Isolate {
    type Image = IsolateImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = &'a mut Heap;

    /// Capture one isolate image for the given mode.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Isolate::image(self).map_err(Box::<RuntimeError>::from)
    }

    /// Restore one isolate image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _heap: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        Isolate::restore_image(self, image).map_err(Box::<RuntimeError>::from)
    }
}

impl SnapshotCodec for Isolate {
    type Snapshot = IsolateImage;

    /// Encode one isolate image as one snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(image.clone())
    }

    /// Decode one isolate snapshot back into one image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(snapshot.clone())
    }
}
