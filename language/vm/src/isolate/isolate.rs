use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_core::{Capture, CaptureMode, ImmutableStringPool, SnapshotCodec};
use destack_mir as mir;

use super::{
    ExternalCallContext, ExternalFn, ExternalHandler, GlobalStorage, SchemaRegistry,
    StringInterner, StringRef,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::{Executable, FunctionTable};
use crate::interpreter::{Continuation, ExecutionOutcome, ExecutionOutput, Interpreter};
use crate::options::IsolateOptions;
use crate::snapshot::{ContinuationImage, IsolateImage, IsolateSnapshot};
use destack_heap::{GcStats, Heap, ManagedReference, MemoryContext, SharedSpace, Value};

// isolate id generator for continuation validation
static ISOLATE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// VM isolate with globals and execution state.
pub struct Isolate {
    /// Unique id used to validate continuation ownership.
    isolate_id: u64,
    /// Immutable executable shared by this isolate.
    executable: Arc<Executable>,
    /// Configuration options for this isolate.
    options: IsolateOptions,
    /// String interner for literal storage.
    string_interner: StringInterner,
    /// Global variable storage.
    globals: GlobalStorage,
    /// External function handlers.
    externals: HashMap<String, ExternalFn>,
    /// Runtime ABI storage schemas installed into this isolate.
    schema: SchemaRegistry,
    /// Interpreter engine backing this isolate.
    interpreter: Interpreter,
}

impl fmt::Debug for Isolate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Isolate")
            .field("executable", &self.executable)
            .field("string_interner", &self.string_interner)
            .field("globals", &format!("<{} globals>", self.globals.len()))
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[allow(clippy::arc_with_non_send_sync)]
impl Isolate {
    /// Create a new isolate from one shared immutable image.
    pub fn new(image: Arc<IsolateImage>) -> RuntimeResult<Self> {
        let executable = Arc::new(Executable::new(image.tree.clone(), image.strings.clone())?);
        let mut isolate = Self {
            isolate_id: image.isolate_id,
            executable,
            options: image.options.clone(),
            string_interner: StringInterner::new(&image.tree),
            globals: image.globals.clone(),
            externals: HashMap::new(),
            schema: SchemaRegistry::new()?,
            interpreter: Interpreter::new(),
        };
        isolate
            .string_interner
            .restore_image(&image.string_interner);
        isolate.interpreter = Interpreter::from_image(isolate.functions(), &image.interpreter)?;

        Ok(isolate)
    }

    /// Build a new isolate with default options.
    pub fn build(tree: mir::NodeTree, strings: ImmutableStringPool) -> RuntimeResult<Self> {
        Self::build_with_options(tree, strings, IsolateOptions::default())
    }

    /// Build a new isolate with custom options.
    pub fn build_with_options(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
    ) -> RuntimeResult<Self> {
        let executable = Arc::new(Executable::new(tree, strings)?);
        let string_interner = StringInterner::new(&executable.tree);
        let isolate_id = ISOLATE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Ok(Self {
            isolate_id,
            executable,
            options,
            string_interner,
            globals: GlobalStorage::new(),
            externals: HashMap::new(),
            schema: SchemaRegistry::new()?,
            interpreter: Interpreter::new(),
        })
    }

    /// Initialize isolate globals against one explicit heap.
    pub fn initialize(&mut self, memory: &mut MemoryContext<'_>) -> RuntimeResult<()> {
        // initialize globals and interned literals
        self.interpreter.initialize_globals(
            self.executable.as_ref(),
            &mut self.string_interner,
            &mut self.globals,
            memory.reborrow(),
        )
    }

    /// Get the isolate options.
    pub fn options(&self) -> &IsolateOptions {
        &self.options
    }

    /// Get mutable isolate options.
    pub fn options_mut(&mut self) -> &mut IsolateOptions {
        &mut self.options
    }

    /// Return the managed-reference width required by this isolate heap.
    pub fn heap_managed_reference_bytes(&self) -> u8 {
        self.executable
            .tree
            .metadata
            .layout
            .storage
            .managed_reference_layout
            .bytes
    }

    /// Set whether to collect execution statistics.
    pub fn set_collect_stats(&mut self, collect: bool) {
        self.options.telemetry.collect_stats = collect;
    }

    /// Enable instruction profiling with the given sampling interval.
    #[cfg(feature = "stats")]
    pub fn enable_instruction_profile(&mut self, sample_interval: std::time::Duration) {
        self.interpreter.enable_instruction_profile(sample_interval);
    }

    /// Reset instruction profiling samples without disabling sampling.
    #[cfg(feature = "stats")]
    pub fn reset_instruction_profile(&mut self) {
        self.interpreter.reset_instruction_profile();
    }

    /// Clear instruction profiling data and disable sampling.
    #[cfg(feature = "stats")]
    pub fn clear_instruction_profile(&mut self) {
        self.interpreter.clear_instruction_profile();
    }

    /// Return a compact instruction profile report if available.
    #[cfg(feature = "stats")]
    pub fn instruction_profile_report(&mut self, target_percent: f64) -> Option<String> {
        self.interpreter.instruction_profile_report(target_percent)
    }

    /// Register a VM binding handler.
    pub fn register_vm_binding(&mut self, name: &str, handler: impl ExternalHandler + 'static) {
        self.externals.insert(name.to_string(), Arc::new(handler));
    }

    /// Register one runtime named type for external ABI fallback.
    pub fn register_named_storage_type(
        &mut self,
        name: &str,
        component_count: usize,
    ) -> Result<(), Error> {
        self.schema
            .register_named_storage_type(name, component_count)
    }

    /// Run a callback with a runtime context for this isolate.
    pub fn with_runtime_context<F, R>(&mut self, memory: &mut MemoryContext<'_>, run: F) -> R
    where
        F: for<'ctx> FnOnce(&mut ExternalCallContext<'ctx>) -> R,
    {
        // borrow the isolate state needed by the external context
        let executable = self.executable.as_ref();
        let schema = &self.schema;
        let string_interner = &mut self.string_interner;
        let memory = memory.reborrow();
        let mut context = ExternalCallContext::new(executable, schema, string_interner, memory);
        run(&mut context)
    }

    /// Intern a UTF-8 string and return the managed string value.
    pub fn intern_string(&mut self, heap: &mut Heap, value: &str) -> Result<Value, Error> {
        self.string_interner.intern_string_literal(heap, value)
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(
        &self,
        name: &str,
    ) -> Result<mir::LocalNodeId<mir::Function>, RuntimeError> {
        let func_id = self.lookup_function_id(name).ok_or_else(|| {
            self.make_error(Error::ExternalFunctionNotFound {
                name: name.to_string(),
            })
        })?;

        Ok(func_id)
    }

    /// Run a function by name and return its output.
    pub fn run_function_by_name(
        &mut self,
        memory: &mut MemoryContext<'_>,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.interpreter.run_function_by_name(
            self.isolate_id,
            self.executable.as_ref(),
            &self.options,
            &self.schema,
            &mut self.string_interner,
            &mut self.globals,
            &self.externals,
            memory,
            name,
            arguments,
        )
    }

    /// Run a function by name and allow yielding.
    pub fn run_function_by_name_yielding(
        &mut self,
        memory: &mut MemoryContext<'_>,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        self.interpreter.run_function_by_name_yielding(
            self.isolate_id,
            self.executable.as_ref(),
            &self.options,
            &self.schema,
            &mut self.string_interner,
            &mut self.globals,
            &self.externals,
            memory,
            name,
            arguments,
        )
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        memory: &mut MemoryContext<'_>,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.interpreter.run_function(
            self.isolate_id,
            self.executable.as_ref(),
            &self.options,
            &self.schema,
            &mut self.string_interner,
            &mut self.globals,
            &self.externals,
            memory,
            func_id,
            arguments,
        )
    }

    /// Run a function by id and allow yielding.
    pub fn run_function_yielding(
        &mut self,
        memory: &mut MemoryContext<'_>,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        self.interpreter.run_function_yielding(
            self.isolate_id,
            self.executable.as_ref(),
            &self.options,
            &self.schema,
            &mut self.string_interner,
            &mut self.globals,
            &self.externals,
            memory,
            func_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    pub fn resume(
        &mut self,
        memory: &mut MemoryContext<'_>,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<ExecutionOutcome> {
        self.interpreter.resume(
            self.isolate_id,
            self.executable.as_ref(),
            &self.options,
            &self.schema,
            &mut self.string_interner,
            &mut self.globals,
            &self.externals,
            memory,
            continuation,
            resume_value,
        )
    }

    /// Capture one continuation as one immutable image.
    pub fn continuation_image(
        &self,
        continuation: &Continuation,
    ) -> RuntimeResult<ContinuationImage> {
        continuation.image(&self.executable)
    }

    /// Restore one continuation from one immutable image.
    pub fn restore_continuation_image(
        &self,
        image: &ContinuationImage,
    ) -> RuntimeResult<Continuation> {
        Continuation::from_image(image, &self.executable, self.functions())
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, heap: &Heap, value: Value) -> Result<String, Error> {
        self.string_interner.string_value(heap, value)
    }

    /// Read a UTF-8 string view from the heap.
    pub fn string_value_ref<'a>(
        &'a self,
        heap: &'a Heap,
        value: Value,
    ) -> Result<StringRef<'a>, Error> {
        self.string_interner.string_value_ref(heap, value)
    }

    /// Read a UTF-8 string from a managed handle.
    pub fn string_value_for_handle(
        &self,
        heap: &Heap,
        handle: ManagedReference,
    ) -> Result<String, Error> {
        self.string_interner.string_value_for_handle(heap, handle)
    }

    /// Collect garbage from managed heap.
    pub fn collect_garbage(
        &mut self,
        heap: &mut Heap,
        shared: &mut SharedSpace,
    ) -> RuntimeResult<GcStats> {
        let mut memory = MemoryContext::new(heap, shared);
        self.interpreter.collect_garbage(
            &self.executable,
            &mut self.string_interner,
            &self.globals,
            memory.reborrow(),
        )
    }

    /// Collect garbage with extra roots from continuations.
    pub fn collect_garbage_with_continuations(
        &mut self,
        heap: &mut Heap,
        shared: &mut SharedSpace,
        continuations: &[Continuation],
    ) -> RuntimeResult<GcStats> {
        let mut memory = MemoryContext::new(heap, shared);
        self.interpreter.collect_garbage_with_continuations(
            &self.executable,
            &mut self.string_interner,
            &self.globals,
            memory.reborrow(),
            continuations,
        )
    }

    /// Capture one immutable VM image.
    pub fn image(&mut self) -> RuntimeResult<IsolateImage> {
        // capture the mutable isolate state
        let string_interner = self.string_interner.image();
        let globals = self.globals.clone();
        let interpreter = self.interpreter.image();

        Ok(IsolateImage {
            tree: self.executable.tree.clone(),
            strings: self.executable.strings.clone(),
            options: self.options.clone(),
            isolate_id: self.isolate_id,
            string_interner,
            globals,
            interpreter,
        })
    }

    /// Restore this isolate from one immutable VM image.
    pub fn restore_image(&mut self, heap: &mut Heap, image: &IsolateImage) -> RuntimeResult<()> {
        self.executable = Arc::new(Executable::new(image.tree.clone(), image.strings.clone())?);
        self.options = image.options.clone();

        // restore isolate-owned mutable state first
        self.isolate_id = image.isolate_id;
        self.string_interner.restore_image(&image.string_interner);
        self.globals = image.globals.clone();

        // rebuild interpreter state over the restored isolate
        let _ = heap;
        self.interpreter = Interpreter::from_image(self.functions(), &image.interpreter)?;

        Ok(())
    }

    /// Capture one serialized VM snapshot.
    pub fn snapshot(&mut self) -> RuntimeResult<IsolateSnapshot> {
        Ok(IsolateSnapshot {
            image: self.image()?,
        })
    }

    /// Restore this isolate from one serialized VM snapshot.
    pub fn restore_snapshot(
        &mut self,
        heap: &mut Heap,
        snapshot: &IsolateSnapshot,
    ) -> RuntimeResult<()> {
        self.restore_image(heap, &snapshot.image)
    }

    // build a runtime error with the current call stack
    fn make_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info())
    }

    // collect call stack info for error reporting
    fn get_call_stack_info(&self) -> Vec<crate::diagnostic::FrameInfo> {
        self.interpreter
            .call_stack()
            .iter()
            .map(|f| {
                let func = self.executable.tree.get(f.function);
                let name = self.executable.strings.get(func.name).to_string();
                crate::diagnostic::FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Resolve a function id by name from the executable.
    pub(crate) fn lookup_function_id(&self, name: &str) -> Option<mir::LocalNodeId<mir::Function>> {
        self.executable.function_id_by_name.get(name).copied()
    }

    /// Borrow the lowered function table.
    pub(crate) fn functions(&self) -> &FunctionTable {
        &self.executable.functions
    }

    /// Borrow the executable MIR tree.
    #[cfg(test)]
    pub(crate) fn tree(&self) -> &mir::NodeTree {
        &self.executable.tree
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
    type Snapshot = IsolateSnapshot;

    /// Encode one isolate image as one snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(IsolateSnapshot {
            image: image.clone(),
        })
    }

    /// Decode one isolate snapshot back into one image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(snapshot.image.clone())
    }
}
