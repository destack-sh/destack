use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "stats")]
use std::time::Duration;

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use crate::diagnostic::{DiagnosticAnchor, Error, FrameInfo, RuntimeError};
use crate::memory::{HeapHandle, ManagedHeap, RawHeap, Value};

use super::decode::thread_function;
#[cfg(feature = "stats")]
use super::statistics::InstructionProfile;
use super::threaded::{CopyRange, INVALID_FUNCTION_INDEX, ThreadedFunction};
use super::{Frame, GlobalStorage, MachineOptions, Statistics};

/// External function type.
pub type ExternalFn = Box<dyn Fn(&[Value]) -> Result<Value, Error> + Send + Sync>;

/// Cached external handler pointer.
type ExternalFnPtr = NonNull<dyn Fn(&[Value]) -> Result<Value, Error> + Send + Sync>;

/// Interpreter id generator for continuation validation.
static INTERPRETER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Output from executing MIR code.
#[derive(Debug, Clone)]
pub struct ExecutionOutput {
    /// The return value of the executed function.
    pub value: Value,
    /// Statistics from this execution.
    pub statistics: Statistics,
    /// Number of managed heap cells at end of execution.
    pub heap_cells: usize,
    /// Number of raw heap cells at end of execution.
    pub raw_heap_cells: usize,
}

/// Resume state captured at a yield terminator.
#[derive(Debug, Clone)]
pub(super) struct YieldState {
    /// Frame index to resume execution in.
    pub frame_index: usize,
    /// Resume block index in the threaded function.
    pub resume_block: u32,
    /// Copy plan for resume arguments.
    pub resume_copies: CopyRange,
    /// Destination for the resumed value.
    pub resume_value: mir::Value,
}

/// Continuation snapshot captured at a yield terminator.
#[derive(Debug)]
pub struct Continuation {
    /// The interpreter id used to validate the continuation.
    pub(super) interpreter_id: u64,
    /// The call stack for the suspended execution.
    pub(super) call_stack: Vec<Frame>,
    /// The SSA value stack for the suspended execution.
    pub(super) value_stack: Vec<Value>,
    /// The local variable stack for the suspended execution.
    pub(super) local_stack: Vec<Value>,
    /// The resume state captured at the yield point.
    pub(super) yield_state: YieldState,
    /// The statistics captured for the suspended execution.
    pub(super) statistics: Statistics,
    /// The instruction profile state for the suspended execution.
    #[cfg(feature = "stats")]
    pub(super) instruction_profile: Option<InstructionProfile>,
}

impl Continuation {
    /// Clone this continuation for multi-shot resumption.
    pub fn clone_for_fork(&self) -> Self {
        let call_stack = self.call_stack.iter().map(Frame::clone_for_fork).collect();
        let value_stack = self.value_stack.clone();
        let local_stack = self.local_stack.clone();
        let yield_state = self.yield_state.clone();
        let statistics = self.statistics.clone();
        #[cfg(feature = "stats")]
        let instruction_profile = self.instruction_profile.clone();

        Self {
            interpreter_id: self.interpreter_id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics,
            #[cfg(feature = "stats")]
            instruction_profile,
        }
    }

    /// Collect managed heap roots referenced by this continuation.
    pub fn collect_roots(&self, roots: &mut Vec<HeapHandle>) {
        // collect roots from captured frames
        for frame in &self.call_stack {
            frame.collect_roots(&self.value_stack, &self.local_stack, roots);
        }
    }
}

/// Yield result from a suspended coroutine execution.
#[derive(Debug)]
pub struct ExecutionYield {
    /// The value yielded to the caller.
    pub value: Value,
    /// The continuation used to resume execution.
    pub continuation: Continuation,
}

/// Outcome from a coroutine-capable execution entry.
#[derive(Debug)]
pub enum ExecutionOutcome {
    /// Execution completed with a final result.
    Completed {
        /// Completed execution output.
        output: ExecutionOutput,
    },
    /// Execution suspended with a yielded value.
    Yielded {
        /// Yield information for the suspended execution.
        yielded: ExecutionYield,
    },
}

/// MIR interpreter using direct-threaded dispatch for fast execution.
///
/// The interpreter pre-compiles all MIR functions into a threaded form at
/// construction time, enabling efficient dispatch via tail calls between
/// instruction handlers.
pub struct Interpreter {
    /// Unique id used to validate continuation ownership.
    pub(super) id: u64,
    /// The MIR tree being executed.
    pub tree: mir::NodeTree,
    /// String pool for names.
    pub strings: ImmutableStringPool,
    /// The managed heap (GC-tracked allocations).
    pub(super) managed_heap: ManagedHeap,
    /// The raw heap (manually managed allocations).
    pub(super) raw_heap: RawHeap,
    /// Global variable storage.
    pub(super) globals: GlobalStorage,
    /// External function handlers.
    pub(super) externals: HashMap<String, ExternalFn>,
    /// Cached external handlers by function id.
    pub(super) externals_by_id: Vec<Option<ExternalFnPtr>>,
    /// Lookup table for function ids by name.
    pub(super) function_name_map: HashMap<String, mir::LocalNodeId<mir::Function>>,
    /// Configuration options.
    pub(super) options: MachineOptions,
    /// Pre-threaded functions for fast dispatch.
    pub(super) threaded_functions: ThreadedFunctionTable,
    /// Explicit call stack (used for GC roots and error reporting).
    pub(super) call_stack: Vec<Frame>,
    /// SSA value stack for all active frames.
    pub(super) value_stack: Vec<Value>,
    /// Local variable stack for all active frames.
    pub(super) local_stack: Vec<Value>,
    /// Execution statistics.
    pub statistics: Statistics,
    /// Optional instruction profiling sampler.
    #[cfg(feature = "stats")]
    pub(super) instruction_profile: Option<InstructionProfile>,
}

/// Threaded function registry for fast lookup.
pub(super) struct ThreadedFunctionTable {
    /// Threaded functions by dense index.
    functions: Vec<ThreadedFunction>,
    /// Mapping from function id to threaded index (INVALID_FUNCTION_INDEX if missing).
    index_by_id: Vec<u32>,
    /// Import status by function id.
    is_import_by_id: Vec<bool>,
}

impl ThreadedFunctionTable {
    /// Build a threaded function table for the MIR tree.
    pub(super) fn new(tree: &mir::NodeTree) -> Self {
        // size tables using the max function id
        let mut max_id = 0usize;
        for (func_id, _) in tree.iter_nodes::<mir::Function>() {
            max_id = max_id.max(func_id.id as usize);
        }

        // collect threadable function ids and import flags
        let mut function_ids = Vec::new();
        let mut is_import_by_id = vec![false; max_id + 1];
        for (func_id, func) in tree.iter_nodes::<mir::Function>() {
            // record import status
            is_import_by_id[func_id.id as usize] = func.is_import();

            // skip imports and declarations without entry blocks
            if func.is_import() || func.entry.is_none() {
                continue;
            }
            function_ids.push(func_id);
        }

        // build id to index mapping
        let mut index_by_id = vec![INVALID_FUNCTION_INDEX; max_id + 1];
        for (index, func_id) in function_ids.iter().enumerate() {
            index_by_id[func_id.id as usize] = index as u32;
        }

        // thread all functions
        let mut functions = Vec::with_capacity(function_ids.len());
        for func_id in &function_ids {
            let threaded = thread_function(tree, *func_id, &index_by_id)
                .unwrap_or_else(|| panic!("failed to thread function: {func_id:?}"));
            functions.push(threaded);
        }

        // assemble table
        Self {
            functions,
            index_by_id,
            is_import_by_id,
        }
    }

    /// Resolve a threaded function index for the given id.
    pub(super) fn index_for(&self, func_id: mir::LocalNodeId<mir::Function>) -> Option<u32> {
        // look up raw index
        let index = self.index_by_id.get(func_id.id as usize).copied()?;

        // reject invalid entries
        if index == INVALID_FUNCTION_INDEX {
            return None;
        }

        // return valid index
        Some(index)
    }

    /// Get a threaded function by index.
    pub(super) fn get_by_index(&self, index: u32) -> Option<&ThreadedFunction> {
        self.functions.get(index as usize)
    }

    /// Get a threaded function pointer by index.
    pub(super) fn get_ptr_by_index(&self, index: u32) -> Option<NonNull<ThreadedFunction>> {
        self.functions.get(index as usize).map(NonNull::from)
    }

    /// Report whether a function id references an import.
    pub(super) fn is_import(&self, func_id: mir::LocalNodeId<mir::Function>) -> bool {
        self.is_import_by_id
            .get(func_id.id as usize)
            .copied()
            .unwrap_or(false)
    }
}

impl std::fmt::Debug for Interpreter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interpreter")
            .field("managed_heap", &self.managed_heap)
            .field("raw_heap", &self.raw_heap)
            .field("globals", &format!("<{} globals>", self.globals.len()))
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("options", &self.options)
            .field("call_stack_depth", &self.call_stack.len())
            .field("statistics", &self.statistics)
            .finish_non_exhaustive()
    }
}

impl Interpreter {
    /// Create a new interpreter with default options.
    pub fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Self {
        Self::with_options(tree, strings, MachineOptions::default())
    }

    /// Create a new interpreter with custom options.
    ///
    /// This pre-compiles all MIR functions into threaded form for fast execution.
    pub fn with_options(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: MachineOptions,
    ) -> Self {
        let mut managed_heap = ManagedHeap::new();
        let globals = Self::initialize_globals(&tree, &mut managed_heap);
        let function_name_map = Self::build_function_name_map(&tree, &strings);

        // pre-thread all functions for fast dispatch
        let threaded_functions = ThreadedFunctionTable::new(&tree);

        // assign a unique interpreter id
        let id = INTERPRETER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            id,
            tree,
            strings,
            managed_heap,
            raw_heap: RawHeap::new(),
            globals,
            externals: HashMap::new(),
            externals_by_id: Vec::new(),
            function_name_map,
            options,
            threaded_functions,
            call_stack: Vec::new(),
            value_stack: Vec::new(),
            local_stack: Vec::new(),
            statistics: Statistics::new(),
            #[cfg(feature = "stats")]
            instruction_profile: None,
        }
    }

    /// Set whether to collect execution statistics.
    pub fn set_collect_stats(&mut self, collect: bool) {
        self.options.collect_stats = collect;
    }

    /// Enable instruction profiling with the given sampling interval.
    #[cfg(feature = "stats")]
    pub fn enable_instruction_profile(&mut self, sample_interval: Duration) {
        self.instruction_profile = Some(InstructionProfile::new(sample_interval));
    }

    /// Reset instruction profiling samples without disabling sampling.
    #[cfg(feature = "stats")]
    pub fn reset_instruction_profile(&mut self) {
        if let Some(profile) = self.instruction_profile.as_mut() {
            profile.reset();
        }
    }

    /// Clear instruction profiling data and disable sampling.
    #[cfg(feature = "stats")]
    pub fn clear_instruction_profile(&mut self) {
        self.instruction_profile = None;
    }

    /// Return a compact instruction profile report if available.
    #[cfg(feature = "stats")]
    pub fn instruction_profile_report(&self, target_percent: f64) -> Option<String> {
        self.instruction_profile
            .as_ref()
            .map(|profile| profile.summary_target(target_percent).format_compact())
    }

    /// Initialize global variables from the MIR tree.
    fn initialize_globals(tree: &mir::NodeTree, heap: &mut ManagedHeap) -> GlobalStorage {
        let mut globals = GlobalStorage::new();

        for (id, global) in tree.iter_nodes::<mir::Global>() {
            // skip imported globals (they need separate registration)
            if global.is_import() {
                continue;
            }

            let value = match &global.initializer {
                Some(init) => Self::convert_initializer(tree, heap, init, global.ty),
                None => Value::VOID,
            };
            globals.set(id, value);
        }

        globals
    }

    /// Build the function name lookup table.
    fn build_function_name_map(
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
    ) -> HashMap<String, mir::LocalNodeId<mir::Function>> {
        // collect names into a lookup map
        let mut map = HashMap::new();
        for (id, func) in tree.iter_nodes::<mir::Function>() {
            let name = strings.get(func.name).to_string();
            map.entry(name).or_insert(id);
        }

        // return lookup map
        map
    }

    /// Convert a global initializer to a runtime value.
    fn convert_initializer(
        tree: &mir::NodeTree,
        heap: &mut ManagedHeap,
        init: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Value {
        match init {
            mir::GlobalInitializer::Zero => Self::zero_value(tree, heap, ty),
            mir::GlobalInitializer::Scalar(constant) => constant.into(),
            mir::GlobalInitializer::Bytes(bytes) => {
                // convert bytes to an aggregate of u8 values
                let values: Vec<Value> = bytes.iter().map(|&b| Value::uint(b as u64, 8)).collect();
                let handle = heap.allocate_with_values(values);
                Value::aggregate(handle)
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                // recursively convert each element
                let values: Vec<Value> = elements
                    .iter()
                    .map(|e| Self::convert_initializer(tree, heap, e, ty))
                    .collect();
                let handle = heap.allocate_with_values(values);
                Value::aggregate(handle)
            }
        }
    }

    /// Create a zero value for a given type.
    fn zero_value(
        tree: &mir::NodeTree,
        heap: &mut ManagedHeap,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Value {
        let ty_node = tree.get(ty);
        match ty_node {
            mir::Type::Int { width, signed } => {
                if *signed {
                    Value::int(0, *width as u8)
                } else {
                    Value::uint(0, *width as u8)
                }
            }
            mir::Type::Float { width } => {
                if *width == 32 {
                    Value::float32(0.0)
                } else {
                    Value::float64(0.0)
                }
            }
            mir::Type::Boolean => Value::bool(false),
            mir::Type::Tuple {
                elements,
                copyability: _,
            } => {
                let values: Vec<Value> = elements
                    .iter()
                    .map(|e| Self::zero_value(tree, heap, *e))
                    .collect();
                let handle = heap.allocate_with_values(values);
                Value::aggregate(handle)
            }
            mir::Type::Array {
                element,
                length,
                copyability: _,
            } => {
                let elem_zero = Self::zero_value(tree, heap, *element);
                let values: Vec<Value> = (0..*length).map(|_| elem_zero).collect();
                let handle = heap.allocate_with_values(values);
                Value::aggregate(handle)
            }
            // for other types (pointers, functions, void, etc.), just use Void
            _ => Value::VOID,
        }
    }

    /// Register an external function handler.
    pub fn register_external<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[Value]) -> Result<Value, Error> + Send + Sync + 'static,
    {
        self.externals.insert(name.to_string(), Box::new(handler));

        // cache handler pointer for direct id lookup
        if let Some(func_id) = self.function_name_map.get(name).copied() {
            let index = func_id.id as usize;
            if self.externals_by_id.len() <= index {
                self.externals_by_id.resize(index + 1, None);
            }
            if let Some(handler) = self.externals.get(name) {
                self.externals_by_id[index] = Some(NonNull::from(handler.as_ref()));
            }
        }
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(
        &self,
        name: &str,
    ) -> Result<mir::LocalNodeId<mir::Function>, RuntimeError> {
        // look up function id
        let func_id = self.function_name_map.get(name).copied().ok_or_else(|| {
            self.make_error(Error::ExternalFunctionNotFound {
                name: name.to_string(),
            })
        })?;

        // return function id
        Ok(func_id)
    }

    /// Resolve an external handler for an imported function id.
    pub(super) fn external_for_id(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Result<ExternalFnPtr, RuntimeError> {
        let index = function_id.id as usize;
        if let Some(handler) = self.externals_by_id.get(index).copied().flatten() {
            return Ok(handler);
        }

        if self.externals_by_id.len() <= index {
            self.externals_by_id.resize(index + 1, None);
        }

        let func = self.tree.get(function_id);
        let name = self.strings.get(func.name).to_string();
        let handler = self
            .externals
            .get(&name)
            .ok_or_else(|| self.make_error(Error::ExternalFunctionNotFound { name }))?;
        let handler_ptr = NonNull::from(handler.as_ref());
        self.externals_by_id[index] = Some(handler_ptr);

        Ok(handler_ptr)
    }

    /// Create an error with current call stack.
    #[cold]
    pub(super) fn make_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info())
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        let handle = self.managed_heap.allocate_with_values(values);
        Value::aggregate(handle)
    }

    /// Allocate a 2-element aggregate on the heap (avoids Vec allocation).
    #[inline]
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        let handle = self.managed_heap.allocate_pair(first, second);
        Value::aggregate(handle)
    }

    /// Allocate a 1-element aggregate on the heap (avoids Vec allocation).
    #[inline]
    pub fn allocate_single(&mut self, value: Value) -> Value {
        let handle = self.managed_heap.allocate_single(value);
        Value::aggregate(handle)
    }

    /// Get the slots of an aggregate value (looking up from heap if needed).
    pub(super) fn get_aggregate_slots(&self, value: &Value) -> Option<&[Value]> {
        value
            .as_heap_handle()
            .and_then(|handle| self.managed_heap.get(handle))
            .map(|cell| cell.slots.as_slice())
    }

    /// Create an error with instruction anchor.
    #[cold]
    #[allow(dead_code)]
    pub(super) fn make_error_at(
        &self,
        error: Error,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> RuntimeError {
        let frame = self.call_stack.last();
        let anchor = if let Some(f) = frame {
            DiagnosticAnchor::Instruction {
                function: f.function,
                block: f.current_block,
                instruction: instruction_id,
            }
        } else {
            DiagnosticAnchor::None
        };

        RuntimeError::new(error)
            .with_call_stack(self.get_call_stack_info())
            .with_anchor(anchor)
    }

    /// Get call stack info for error reporting.
    fn get_call_stack_info(&self) -> Vec<FrameInfo> {
        self.call_stack
            .iter()
            .map(|f| {
                let func = self.tree.get(f.function);
                let name = self.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Get a reference to the managed heap.
    pub fn managed_heap(&self) -> &ManagedHeap {
        &self.managed_heap
    }

    /// Get a mutable reference to the managed heap.
    pub fn managed_heap_mut(&mut self) -> &mut ManagedHeap {
        &mut self.managed_heap
    }

    /// Get a reference to the raw heap.
    pub fn raw_heap(&self) -> &RawHeap {
        &self.raw_heap
    }

    /// Get a mutable reference to the raw heap.
    pub fn raw_heap_mut(&mut self) -> &mut RawHeap {
        &mut self.raw_heap
    }

    /// Run garbage collection on the managed heap.
    pub fn collect_garbage(&mut self) -> GcStats {
        self.collect_garbage_with_continuations(&[])
    }

    /// Run garbage collection including suspended continuations.
    pub fn collect_garbage_with_continuations(
        &mut self,
        continuations: &[Continuation],
    ) -> GcStats {
        // collect roots from active frames
        let mut roots = Vec::new();
        for frame in &self.call_stack {
            frame.collect_roots(&self.value_stack, &self.local_stack, &mut roots);
        }

        // collect roots from continuations
        for continuation in continuations {
            continuation.collect_roots(&mut roots);
        }

        // run collection
        let freed_cells = self.managed_heap.collect(&roots);
        let live_cells = self.managed_heap.cell_count();

        GcStats {
            freed_cells,
            live_cells,
        }
    }
}

/// Summary statistics for a garbage collection cycle.
#[derive(Clone, Copy, Debug, Default)]
pub struct GcStats {
    /// Number of cells freed by the collection.
    pub freed_cells: usize,
    /// Number of live cells after the collection.
    pub live_cells: usize,
}
