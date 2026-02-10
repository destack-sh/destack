use std::fmt;

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use super::{ExternalCallContext, ExternalHandler, IsolateState, StringRef};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::execute::{Continuation, ExecutionOutcome, ExecutionOutput};
use crate::interpreter::{Interpreter, InterpreterContext};
use crate::memory::{GcStats, HeapHandle, RawPointer, SharedHeap, Value};
use crate::options::IsolateOptions;

/// VM isolate with its own heaps, globals, and execution state.
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
    /// Create a new isolate with default options.
    pub fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> RuntimeResult<Self> {
        Self::with_options(tree, strings, IsolateOptions::default())
    }

    /// Create a new isolate with custom options.
    pub fn with_options(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
    ) -> RuntimeResult<Self> {
        let mut state = IsolateState::new(tree, strings, options);
        let mut interpreter = Interpreter::new(&state);
        // initialize globals and interned literals
        {
            let mut context = interpreter.context(&mut state);
            context.initialize_globals()?;
        }

        Ok(Self { state, interpreter })
    }

    /// Create a new isolate with custom options and an explicit heap store.
    pub fn with_options_and_heap_store(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
        heap: SharedHeap,
    ) -> RuntimeResult<Self> {
        let mut state = IsolateState::new_with_heap_store(tree, strings, options, heap);
        let mut interpreter = Interpreter::new(&state);
        // initialize globals and interned literals
        {
            let mut context = interpreter.context(&mut state);
            context.initialize_globals()?;
        }

        Ok(Self { state, interpreter })
    }

    /// Get the isolate options.
    pub fn options(&self) -> &IsolateOptions {
        &self.state.options
    }

    /// Get mutable isolate options.
    pub fn options_mut(&mut self) -> &mut IsolateOptions {
        &mut self.state.options
    }

    /// Set whether to collect execution statistics.
    pub fn set_collect_stats(&mut self, collect: bool) {
        self.state.options.telemetry.collect_stats = collect;
    }

    /// Enable instruction profiling with the given sampling interval.
    #[cfg(feature = "stats")]
    pub fn enable_instruction_profile(&mut self, sample_interval: std::time::Duration) {
        self.with_interpreter(|context| context.enable_instruction_profile(sample_interval));
    }

    /// Reset instruction profiling samples without disabling sampling.
    #[cfg(feature = "stats")]
    pub fn reset_instruction_profile(&mut self) {
        self.with_interpreter(|context| context.reset_instruction_profile());
    }

    /// Clear instruction profiling data and disable sampling.
    #[cfg(feature = "stats")]
    pub fn clear_instruction_profile(&mut self) {
        self.with_interpreter(|context| context.clear_instruction_profile());
    }

    /// Return a compact instruction profile report if available.
    #[cfg(feature = "stats")]
    pub fn instruction_profile_report(&mut self, target_percent: f64) -> Option<String> {
        self.with_interpreter(|context| context.instruction_profile_report(target_percent))
    }

    /// Register a VM binding handler.
    pub fn register_vm_binding(&mut self, name: &str, handler: impl ExternalHandler + 'static) {
        self.state.register_vm_binding(name, handler);
    }

    /// Run a callback with a runtime context for this isolate.
    pub fn with_runtime_context<F, R>(&mut self, run: F) -> R
    where
        F: for<'ctx> FnOnce(&mut ExternalCallContext<'ctx>) -> R,
    {
        let mut context = ExternalCallContext::new(&mut self.state);
        run(&mut context)
    }

    /// Intern a UTF-8 string and return the managed string value.
    pub fn intern_string(&mut self, value: &str) -> Value {
        self.state.intern_string_literal(value)
    }

    /// Allocate a raw heap cell with value slots and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.state.allocate_raw_values(values)
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
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.with_interpreter(|context| context.run_function_by_name(name, arguments))
    }

    /// Run a function by name and allow yielding.
    pub fn run_function_by_name_yielding(
        &mut self,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        self.with_interpreter(|context| context.run_function_by_name_yielding(name, arguments))
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.with_interpreter(|context| context.run_function(func_id, arguments))
    }

    /// Run a function by id and allow yielding.
    pub fn run_function_yielding(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutcome> {
        self.with_interpreter(|context| context.run_function_yielding(func_id, arguments))
    }

    /// Resume a previously yielded coroutine.
    pub fn resume(
        &mut self,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<ExecutionOutcome> {
        self.with_interpreter(|context| context.resume(continuation, resume_value))
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        self.state.allocate_aggregate(values)
    }

    /// Allocate a 2-element aggregate on the heap (avoids Vec allocation).
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        self.state.allocate_pair(first, second)
    }

    /// Allocate a 1-element aggregate on the heap (avoids Vec allocation).
    pub fn allocate_single(&mut self, value: Value) -> Value {
        self.state.allocate_single(value)
    }

    /// Get a shared heap handle.
    pub fn heap(&self) -> SharedHeap {
        self.state.heap.clone()
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, value: Value) -> Result<String, Error> {
        self.state.string_value(value)
    }

    /// Read a UTF-8 string view from the heap.
    pub fn string_value_ref(&self, value: Value) -> Result<StringRef<'_>, Error> {
        self.state.string_value_ref(value)
    }

    /// Read a UTF-8 string from a managed handle.
    pub fn string_value_for_handle(&self, handle: HeapHandle) -> Result<String, Error> {
        self.state.string_value_for_handle(handle)
    }

    /// Get a shared heap handle.
    pub fn heap_mut(&mut self) -> SharedHeap {
        self.state.heap.clone()
    }

    /// Collect garbage from managed heap.
    pub fn collect_garbage(&mut self) -> GcStats {
        self.with_interpreter(|context| context.collect_garbage())
    }

    /// Collect garbage with extra roots from continuations.
    pub fn collect_garbage_with_continuations(
        &mut self,
        continuations: &[Continuation],
    ) -> GcStats {
        self.with_interpreter(|context| context.collect_garbage_with_continuations(continuations))
    }

    // interpret a closure with access to the interpreter context
    fn with_interpreter<R>(&mut self, f: impl FnOnce(&mut InterpreterContext<'_>) -> R) -> R {
        let mut context = self.interpreter.context(&mut self.state);
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
                let func = self.state.tree.get(f.function);
                let name = self.state.strings.get(func.name).to_string();
                crate::diagnostic::FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }
}
