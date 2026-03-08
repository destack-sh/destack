use std::fmt;
use std::sync::Arc;

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use super::{ExternalCallContext, ExternalHandler, IsolateState, StringRef};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::execute::{Continuation, ExecutionOutcome, ExecutionOutput};
use crate::interpreter::{Interpreter, InterpreterContext};
use crate::options::IsolateOptions;
use crate::snapshot::{IsolateImage, IsolateSnapshot};
use destack_heap::{GcStats, Heap, ManagedPointer, RawPointer, Value};

/// VM isolate with globals and execution state.
pub struct Isolate {
    /// Shared isolate state for all engines.
    state: IsolateState,
    /// Interpreter engine backing this isolate.
    interpreter: Interpreter,
}

impl fmt::Debug for Isolate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Isolate")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Isolate {
    /// Create a new isolate from one shared immutable image.
    pub fn new(image: Arc<IsolateImage>) -> RuntimeResult<Self> {
        let state = IsolateState::new(image);
        let interpreter = Interpreter::new(&state);

        Ok(Self { state, interpreter })
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
        let image = Arc::new(IsolateImage {
            tree,
            strings,
            options,
        });

        Self::new(image)
    }

    /// Initialize isolate globals against one explicit heap.
    pub fn initialize(&mut self, heap: &mut Heap) -> RuntimeResult<()> {
        // initialize globals and interned literals
        self.with_interpreter(heap, |context| context.initialize_globals())
    }

    /// Get the isolate options.
    pub fn options(&self) -> &IsolateOptions {
        &self.state.image.options
    }

    /// Return the shared immutable isolate construction image.
    pub fn image(&self) -> &Arc<IsolateImage> {
        &self.state.image
    }

    /// Get mutable isolate options.
    pub fn options_mut(&mut self) -> &mut IsolateOptions {
        &mut Arc::make_mut(&mut self.state.image).options
    }

    /// Set whether to collect execution statistics.
    pub fn set_collect_stats(&mut self, collect: bool) {
        Arc::make_mut(&mut self.state.image)
            .options
            .telemetry
            .collect_stats = collect;
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
        self.state.register_vm_binding(name, handler);
    }

    /// Run a callback with a runtime context for this isolate.
    pub fn with_runtime_context<F, R>(&mut self, heap: &mut Heap, run: F) -> R
    where
        F: for<'ctx> FnOnce(&mut ExternalCallContext<'ctx>) -> R,
    {
        let mut context = ExternalCallContext::new(&mut self.state, heap);
        run(&mut context)
    }

    /// Intern a UTF-8 string and return the managed string value.
    pub fn intern_string(&mut self, heap: &mut Heap, value: &str) -> Value {
        self.state.intern_string_literal(heap, value)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub fn allocate_raw_values(&mut self, heap: &mut Heap, values: Vec<Value>) -> RawPointer {
        self.state.allocate_raw_values(heap, values)
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(
        &self,
        name: &str,
    ) -> Result<mir::LocalNodeId<mir::Function>, RuntimeError> {
        let func_id = self
            .state
            .function_name_map
            .get(name)
            .copied()
            .ok_or_else(|| {
                self.make_error(Error::ExternalFunctionNotFound {
                    name: name.to_string(),
                })
            })?;

        Ok(func_id)
    }

    /// Run a function by name and return its output.
    pub fn run_function_by_name(
        &mut self,
        heap: &mut Heap,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.with_interpreter(heap, |context| {
            context.run_function_by_name(name, arguments)
        })
    }

    /// Run a function by name and allow yielding.
    pub fn run_function_by_name_yielding(
        &mut self,
        heap: &mut Heap,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        self.with_interpreter(heap, |context| {
            context.run_function_by_name_yielding(name, arguments)
        })
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        heap: &mut Heap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.with_interpreter(heap, |context| context.run_function(func_id, arguments))
    }

    /// Run a function by id and allow yielding.
    pub fn run_function_yielding(
        &mut self,
        heap: &mut Heap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        self.with_interpreter(heap, |context| {
            context.run_function_yielding(func_id, arguments)
        })
    }

    /// Resume a previously yielded coroutine.
    pub fn resume(
        &mut self,
        heap: &mut Heap,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<ExecutionOutcome> {
        self.with_interpreter(heap, |context| context.resume(continuation, resume_value))
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, heap: &mut Heap, values: Vec<Value>) -> Value {
        self.state.allocate_aggregate(heap, values)
    }

    /// Allocate a 2-element aggregate on the heap.
    pub fn allocate_pair(&mut self, heap: &mut Heap, first: Value, second: Value) -> Value {
        self.state.allocate_pair(heap, first, second)
    }

    /// Allocate a 1-element aggregate on the heap.
    pub fn allocate_single(&mut self, heap: &mut Heap, value: Value) -> Value {
        self.state.allocate_single(heap, value)
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, heap: &Heap, value: Value) -> Result<String, Error> {
        self.state.string_value(heap, value)
    }

    /// Read a UTF-8 string view from the heap.
    pub fn string_value_ref<'a>(
        &'a self,
        heap: &'a Heap,
        value: Value,
    ) -> Result<StringRef<'a>, Error> {
        self.state.string_value_ref(heap, value)
    }

    /// Read a UTF-8 string from a managed handle.
    pub fn string_value_for_handle(
        &self,
        heap: &Heap,
        handle: ManagedPointer,
    ) -> Result<String, Error> {
        self.state.string_value_for_handle(heap, handle)
    }

    /// Collect garbage from managed heap.
    pub fn collect_garbage(&mut self, heap: &mut Heap) -> GcStats {
        self.with_interpreter(heap, |context| context.collect_garbage())
    }

    /// Collect garbage with extra roots from continuations.
    pub fn collect_garbage_with_continuations(
        &mut self,
        heap: &mut Heap,
        continuations: &[Continuation],
    ) -> GcStats {
        self.with_interpreter(heap, |context| {
            context.collect_garbage_with_continuations(continuations)
        })
    }

    /// Capture one durable VM snapshot.
    pub fn snapshot(&mut self) -> RuntimeResult<IsolateSnapshot> {
        // capture the mutable isolate state
        let string_interner = self.state.string_interner.snapshot();
        let globals = self.state.globals.clone();
        let interpreter = self.interpreter.snapshot();

        Ok(IsolateSnapshot {
            isolate_id: self.state.isolate_id,
            string_interner,
            globals,
            interpreter,
        })
    }

    /// Restore this isolate from one durable VM snapshot.
    pub fn restore(&mut self, heap: &mut Heap, snapshot: &IsolateSnapshot) -> RuntimeResult<()> {
        // restore isolate-owned mutable state first
        self.state.isolate_id = snapshot.isolate_id;
        self.state
            .string_interner
            .restore(&snapshot.string_interner);
        self.state.globals = snapshot.globals.clone();

        // rebuild interpreter state over the restored isolate
        self.interpreter = Interpreter::restore(&self.state, heap, &snapshot.interpreter)?;

        Ok(())
    }

    // interpret a closure with access to the interpreter context
    fn with_interpreter<R>(
        &mut self,
        heap: &mut Heap,
        f: impl FnOnce(&mut InterpreterContext<'_>) -> R,
    ) -> R {
        let mut context = self.interpreter.context(&mut self.state, heap);
        f(&mut context)
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
                let func = self.state.image.tree.get(f.function);
                let name = self.state.image.strings.get(func.name).to_string();
                crate::diagnostic::FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }
}
