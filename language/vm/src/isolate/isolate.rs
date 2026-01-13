use std::fmt;

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use super::IsolateState;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::engine::compiled::CompiledEngine;
use crate::engine::interpreter::{InterpreterContext, InterpreterEngine};
use crate::execute::{Continuation, ExecutionOutcome, ExecutionOutput};
use crate::memory::{GcStats, HeapHandle, ManagedHeap, RawHeap, StringLayout, Value};
use crate::options::IsolateOptions;

/// VM isolate with its own heaps, globals, and execution state.
pub struct Isolate {
    /// Shared isolate state for all engines.
    state: IsolateState,
    /// Interpreter engine backing this isolate.
    interpreter: InterpreterEngine,
    /// Compiled engine backing this isolate.
    compiled: CompiledEngine,
}

impl fmt::Debug for Isolate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Isolate")
            .field("state", &self.state)
            .field("compiled", &self.compiled)
            .finish_non_exhaustive()
    }
}

impl Isolate {
    /// Create a new isolate with default options.
    pub fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Self {
        Self::with_options(tree, strings, IsolateOptions::default())
    }

    /// Create a new isolate with custom options.
    pub fn with_options(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
    ) -> Self {
        let mut state = IsolateState::new(tree, strings, options);
        let mut interpreter = InterpreterEngine::new(&state);
        let compiled = CompiledEngine::new(&state);

        // initialize globals and interned literals
        {
            let mut context = interpreter.context(&mut state);
            context.initialize_globals();
            context.pre_intern_threaded_strings();
        }

        Self {
            state,
            interpreter,
            compiled,
        }
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

    /// Register an external function handler.
    pub fn register_external<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[Value]) -> Result<Value, Error> + Send + Sync + 'static,
    {
        self.with_interpreter(|context| context.register_external(name, handler));
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
        self.with_interpreter(|context| context.allocate_aggregate(values))
    }

    /// Allocate a 2-element aggregate on the heap (avoids Vec allocation).
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        self.with_interpreter(|context| context.allocate_pair(first, second))
    }

    /// Allocate a 1-element aggregate on the heap (avoids Vec allocation).
    pub fn allocate_single(&mut self, value: Value) -> Value {
        self.with_interpreter(|context| context.allocate_single(value))
    }

    /// Get a reference to the managed heap.
    pub fn managed_heap(&self) -> &ManagedHeap {
        &self.state.managed_heap
    }

    /// Get a mutable reference to the managed heap.
    pub fn managed_heap_mut(&mut self) -> &mut ManagedHeap {
        &mut self.state.managed_heap
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, value: Value) -> Result<String, Error> {
        let handle = match value.tag() {
            crate::memory::ValueTag::String => value.as_heap_handle().unwrap(),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "string".to_string(),
                    actual: format!("{value:?}"),
                });
            }
        };

        self.string_value_for_handle(handle)
    }

    /// Read a UTF-8 string from a managed handle.
    pub fn string_value_for_handle(&self, handle: HeapHandle) -> Result<String, Error> {
        if handle.is_null() {
            return Err(Error::NullPointerDereference);
        }

        let cell = self
            .state
            .managed_heap
            .get(handle)
            .ok_or(Error::InvalidHeapHandle)?;
        let length_value = cell
            .slots
            .get(StringLayout::LENGTH_BYTES)
            .copied()
            .ok_or(Error::InvalidHeapHandle)?;
        let length = length_value.as_uint().ok_or_else(|| Error::TypeMismatch {
            expected: "u32".to_string(),
            actual: format!("{length_value:?}"),
        })? as usize;
        if length == 0 {
            return Ok(String::new());
        }

        let data_value = cell
            .slots
            .get(StringLayout::DATA)
            .copied()
            .ok_or(Error::InvalidHeapHandle)?;
        let data_ptr = data_value
            .as_raw_pointer()
            .ok_or(Error::InvalidHeapHandle)?;
        let raw_cell = self
            .state
            .raw_heap
            .get(data_ptr)
            .ok_or(Error::InvalidHeapHandle)?;
        let bytes = match &raw_cell.storage {
            crate::memory::RawCellStorage::Bytes(bytes) => bytes,
            _ => return Err(Error::InvalidHeapHandle),
        };
        let value = String::from_utf8(bytes.clone()).map_err(|_| Error::TypeMismatch {
            expected: "string".to_string(),
            actual: "bytes".to_string(),
        })?;

        Ok(value)
    }

    /// Get a reference to the raw heap.
    pub fn raw_heap(&self) -> &RawHeap {
        &self.state.raw_heap
    }

    /// Get a mutable reference to the raw heap.
    pub fn raw_heap_mut(&mut self) -> &mut RawHeap {
        &mut self.state.raw_heap
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
