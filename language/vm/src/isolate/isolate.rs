use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use destack_core::{Capture, CaptureMode, ImmutableStringPool, SnapshotCodec};
use destack_engine::{self as engine, StaticSpace};
use destack_mir as mir;

use super::{ExternalCallContext, ExternalFn, ExternalHandler, RootSet, RootSink};
use crate::diagnostic::{Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::interpreter::{Continuation, Interpreter, Outcome, Output};
use crate::options::IsolateOptions;
use crate::program::Program;
use crate::snapshot::{ContinuationImage, IsolateImage};
use crate::{SharedHeap, Word};
use destack_heap::{
    Heap, HeapReference, HeapResult, RootSlot, SharedAllocator, SharedGcWorker, SharedRawLimits,
};

/// VM isolate with static data and execution state.
pub struct Isolate {
    /// Unique id used to validate continuation ownership.
    isolate_id: engine::EngineId,
    /// Immutable program shared by this isolate.
    program: Arc<Program>,
    /// Configuration options for this isolate.
    options: IsolateOptions,
    /// External function handlers.
    externals: HashMap<String, ExternalFn>,
    /// Interpreter engine backing this isolate.
    interpreter: Interpreter,
    /// Worker allocator for shared managed heap allocation.
    shared_allocator: Option<SharedAllocator>,
    /// Current shared collector worker during one engine call.
    shared_gc: Option<usize>,
}

impl fmt::Debug for Isolate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Isolate")
            .field("program", &self.program)
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[allow(clippy::arc_with_non_send_sync)]
impl Isolate {
    /// Create a new isolate from one shared immutable image.
    pub fn new(image: Arc<IsolateImage>) -> RuntimeResult<Self> {
        let program = Arc::new(Program::new(image.tree.clone(), image.strings.clone())?);
        let mut isolate = Self {
            isolate_id: image.isolate_id,
            program,
            options: image.options.clone(),
            externals: HashMap::new(),
            interpreter: Interpreter::new(&image.options)?,
            shared_allocator: None,
            shared_gc: None,
        };
        isolate.interpreter =
            Interpreter::from_image(&isolate.program, &image.interpreter, &isolate.options)?;

        Ok(isolate)
    }

    /// Build a new isolate with default options.
    pub fn build(
        isolate_id: engine::EngineId,
        tree: mir::Tree,
        strings: ImmutableStringPool,
    ) -> RuntimeResult<Self> {
        Self::build_with_options(isolate_id, tree, strings, IsolateOptions::default())
    }

    /// Build a new isolate with custom options.
    pub fn build_with_options(
        isolate_id: engine::EngineId,
        tree: mir::Tree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
    ) -> RuntimeResult<Self> {
        let program = Arc::new(Program::new(tree, strings)?);
        let native_pointer_bytes = program.tree.metadata.layout.storage.native_pointer_bytes;
        let host_pointer_bytes = HeapReference::BYTE_LEN as u8;

        // host execution only supports native-width pointers
        if native_pointer_bytes != host_pointer_bytes {
            return Err(RuntimeError::new(Error::IncompatiblePointerWidth {
                bytes: native_pointer_bytes,
                host_bytes: host_pointer_bytes,
            }));
        }

        let interpreter = Interpreter::new(&options)?;

        Ok(Self {
            isolate_id,
            program,
            options,
            externals: HashMap::new(),
            interpreter,
            shared_allocator: None,
            shared_gc: None,
        })
    }

    /// Enter one engine call with one shared collector worker.
    pub(crate) fn enter_shared_gc(&mut self, worker: &SharedGcWorker) {
        self.shared_gc = Some(worker as *const SharedGcWorker as usize);
    }

    /// Leave the current engine shared collector worker scope.
    pub(crate) fn leave_shared_gc(&mut self) {
        self.shared_gc = None;
    }

    /// Prepare this isolate's shared heap allocator for one engine call.
    fn prepare_shared_allocator(&mut self, shared: &SharedHeap) {
        if self.shared_allocator.is_none() {
            self.shared_allocator = Some(shared.allocator());
        }

        if let Some(allocator) = self.shared_allocator.as_mut() {
            let shared_gc = self
                .shared_gc
                .map(|worker| unsafe { &*(worker as *const SharedGcWorker) });
            allocator.set_gc_worker(shared_gc);
        }
    }

    /// Publish and retire allocator-local shared heap runs.
    pub fn flush_shared_allocator(&mut self, shared: &SharedHeap) {
        if let Some(allocator) = self.shared_allocator.as_mut() {
            shared.flush_allocator(allocator);
        }
    }

    /// Initialize heap-shaped program metadata and worker static bytes.
    pub fn initialize(
        &mut self,
        heap: &Heap,
        shared: &SharedHeap,
        statics: &mut StaticSpace,
    ) -> RuntimeResult<()> {
        // lower allocation opcodes for the live heap geometry
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

    /// Get the isolate options.
    pub fn options(&self) -> &IsolateOptions {
        &self.options
    }

    /// Get mutable isolate options.
    pub fn options_mut(&mut self) -> &mut IsolateOptions {
        &mut self.options
    }

    /// Register a VM binding handler.
    pub fn register_vm_binding(&mut self, name: &str, handler: impl ExternalHandler + 'static) {
        self.externals.insert(name.to_string(), Arc::new(handler));
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

    /// Run a callback with a runtime context for this isolate.
    pub fn with_runtime_context<F, R>(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_raw_limits: SharedRawLimits,
        run: F,
    ) -> Result<R, Error>
    where
        F: for<'ctx> FnOnce(&mut ExternalCallContext<'ctx>) -> Result<R, Error>,
    {
        // borrow the isolate state needed by the external context
        let program = self.program.as_ref();
        let mut context = ExternalCallContext::new(program, heap, shared, shared_raw_limits);
        let result = run(&mut context);
        context.release_pins()?;

        result
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(
        &self,
        name: &str,
    ) -> Result<mir::LocalNodeId<mir::Function>, RuntimeError> {
        let func_id = self
            .program
            .function_id_by_name
            .get(name)
            .copied()
            .ok_or_else(|| {
                self.runtime_error(Error::ExternalFunctionNotFound {
                    name: name.to_string(),
                })
            })?;

        Ok(func_id)
    }

    /// Run a function by name and return its output.
    pub fn run_function_by_name(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[engine::Value],
    ) -> RuntimeResult<Output> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_by_name_words(statics, heap, shared, name, &arguments)
    }

    /// Run a function by name with VM words and return its output.
    pub(crate) fn run_function_by_name_words(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Word],
    ) -> RuntimeResult<Output> {
        self.prepare_shared_allocator(shared);
        let Self {
            isolate_id,
            program,
            options,
            externals,
            interpreter,
            shared_allocator,
            ..
        } = self;
        let shared_allocator = shared_allocator
            .as_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        let result = interpreter.run_function_by_name(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            externals,
            heap,
            shared,
            shared_allocator,
            name,
            arguments,
        );
        shared.flush_allocator(shared_allocator);

        result
    }

    /// Run a function by name and allow yielding.
    pub fn run_function_by_name_yielding(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[engine::Value],
    ) -> RuntimeResult<Outcome> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_by_name_yielding_words(statics, heap, shared, name, &arguments)
    }

    /// Run a function by name with VM words and allow yielding.
    pub(crate) fn run_function_by_name_yielding_words(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        self.prepare_shared_allocator(shared);
        let Self {
            isolate_id,
            program,
            options,
            externals,
            interpreter,
            shared_allocator,
            ..
        } = self;
        let shared_allocator = shared_allocator
            .as_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        let result = interpreter.run_function_by_name_yielding(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            externals,
            heap,
            shared,
            shared_allocator,
            name,
            arguments,
        );
        shared.flush_allocator(shared_allocator);

        result
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[engine::Value],
    ) -> RuntimeResult<Output> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_words(statics, heap, shared, func_id, &arguments)
    }

    /// Run a function by id with VM words and return its output.
    pub(crate) fn run_function_words(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Output> {
        self.prepare_shared_allocator(shared);
        let Self {
            isolate_id,
            program,
            options,
            externals,
            interpreter,
            shared_allocator,
            ..
        } = self;
        let shared_allocator = shared_allocator
            .as_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        let result = interpreter.run_function(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            externals,
            heap,
            shared,
            shared_allocator,
            func_id,
            arguments,
        );
        shared.flush_allocator(shared_allocator);

        result
    }

    /// Run a function by id and allow yielding.
    pub fn run_function_yielding(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[engine::Value],
    ) -> RuntimeResult<Outcome> {
        let arguments = arguments.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_yielding_words(statics, heap, shared, func_id, &arguments)
    }

    /// Run a function by id with VM words and allow yielding.
    pub(crate) fn run_function_yielding_words(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Word],
    ) -> RuntimeResult<Outcome> {
        self.prepare_shared_allocator(shared);
        let Self {
            isolate_id,
            program,
            options,
            externals,
            interpreter,
            shared_allocator,
            ..
        } = self;
        let shared_allocator = shared_allocator
            .as_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        let result = interpreter.run_function_yielding(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            externals,
            heap,
            shared,
            shared_allocator,
            func_id,
            arguments,
        );
        shared.flush_allocator(shared_allocator);

        result
    }

    /// Resume a previously yielded coroutine.
    pub fn resume(
        &mut self,
        statics: &mut StaticSpace,
        heap: &mut Heap,
        shared: &SharedHeap,
        continuation: Continuation,
        resume_value: engine::Value,
    ) -> RuntimeResult<Outcome> {
        self.prepare_shared_allocator(shared);
        let Self {
            isolate_id,
            program,
            options,
            externals,
            interpreter,
            shared_allocator,
            ..
        } = self;
        let shared_allocator = shared_allocator
            .as_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        let result = interpreter.resume(
            *isolate_id,
            program.as_ref(),
            options,
            statics,
            externals,
            heap,
            shared,
            shared_allocator,
            continuation,
            resume_value,
        );
        shared.flush_allocator(shared_allocator);

        result
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
        let mut continuation =
            Continuation::from_image(image, &self.program, &self.program.functions, &self.options)?;
        continuation.isolate_id = self.isolate_id;

        Ok(continuation)
    }

    /// Collect one complete root set from live state and optional continuations.
    pub fn root_set(
        &mut self,
        statics: &StaticSpace,
        continuations: &[Continuation],
    ) -> RuntimeResult<RootSet> {
        let mut roots = RootSet::default();

        self.interpreter
            .visit_roots(&self.program, statics, continuations, &mut roots)?;

        Ok(roots)
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

    /// Collect roots from one live continuation.
    pub fn continuation_root_set(&mut self, continuation: &Continuation) -> RuntimeResult<RootSet> {
        let mut roots = RootSet::default();
        self.visit_continuation_roots(continuation, &mut roots)?;

        Ok(roots)
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

    /// Collect roots from one captured continuation image.
    pub fn continuation_image_root_set(
        &mut self,
        image: &ContinuationImage,
    ) -> RuntimeResult<RootSet> {
        let mut roots = RootSet::default();

        self.visit_image_roots(image, &mut roots)?;

        Ok(roots)
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
    pub fn image(&mut self) -> RuntimeResult<IsolateImage> {
        let interpreter = self.interpreter.image();

        Ok(IsolateImage {
            tree: self.program.tree.clone(),
            strings: self.program.strings.clone(),
            options: self.options.clone(),
            isolate_id: self.isolate_id,
            interpreter,
        })
    }

    /// Fork this isolate for one child branch.
    pub fn fork(&self) -> RuntimeResult<Self> {
        Ok(Self {
            isolate_id: self.isolate_id,
            program: self.program.clone(),
            options: self.options.clone(),
            externals: self.externals.clone(),
            interpreter: self.interpreter.fork()?,
            shared_allocator: None,
            shared_gc: None,
        })
    }

    /// Restore this isolate from one immutable VM image.
    pub fn restore_image(&mut self, _heap: &mut Heap, image: &IsolateImage) -> RuntimeResult<()> {
        self.program = Arc::new(Program::new(image.tree.clone(), image.strings.clone())?);
        self.options = image.options.clone();

        self.isolate_id = image.isolate_id;

        // rebuild interpreter state over the restored isolate
        self.interpreter =
            Interpreter::from_image(&self.program, &image.interpreter, &self.options)?;

        Ok(())
    }

    /// Create a runtime error with current call stack.
    fn runtime_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.call_stack())
    }

    /// Return the current call stack for error reporting.
    fn call_stack(&self) -> Vec<FrameInfo> {
        self.interpreter
            .frames()
            .iter()
            .map(|f| {
                let func = self.program.tree.get(f.function);
                let name = self.program.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Borrow the canonical runtime layout table.
    pub fn layout_table(&self) -> &mir::LayoutTable {
        self.program.layouts()
    }

    /// Return the canonical layout id for one MIR type.
    pub fn layout_id_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<mir::LayoutId> {
        self.program.layout_id_for_type(ty)
    }

    /// Return the heap allocation plan for one layout id.
    pub fn allocation_plan(
        &self,
        layout_id: mir::LayoutId,
    ) -> crate::Result<destack_heap::AllocationPlan<'_>> {
        self.program.allocation_plan(layout_id)
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
        heap: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        Isolate::restore_image(self, heap, image).map_err(Box::<RuntimeError>::from)
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
