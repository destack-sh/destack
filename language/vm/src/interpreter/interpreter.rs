use std::collections::HashMap;
use std::sync::Arc;

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use crate::diagnostic::{DiagnosticAnchor, Error, FrameInfo, RuntimeError};
use crate::memory::{ManagedHeap, RawHeap, Value};

use super::decode::thread_function;
use super::threaded::{INVALID_FUNCTION_INDEX, ThreadedFunction};
use super::{Frame, GlobalStorage, MachineOptions, Statistics};

/// External function type.
pub type ExternalFn = Box<dyn Fn(&[Value]) -> Result<Value, Error> + Send + Sync>;

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

/// MIR interpreter using direct-threaded dispatch for fast execution.
///
/// The interpreter pre-compiles all MIR functions into a threaded form at
/// construction time, enabling efficient dispatch via tail calls between
/// instruction handlers.
pub struct Interpreter {
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
}

/// Threaded function registry for fast lookup.
pub(super) struct ThreadedFunctionTable {
    /// Threaded functions by dense index.
    functions: Vec<Arc<ThreadedFunction>>,
    /// Mapping from function id to threaded index (INVALID_FUNCTION_INDEX if missing).
    index_by_id: Vec<u32>,
}

impl ThreadedFunctionTable {
    /// Build a threaded function table for the MIR tree.
    pub(super) fn new(tree: &mir::NodeTree) -> Self {
        // collect threadable function ids
        let mut function_ids = Vec::new();
        let mut max_id = 0usize;
        for (func_id, func) in tree.iter_nodes::<mir::Function>() {
            // skip imports and declarations without entry blocks
            if func.is_import() || func.entry.is_none() {
                continue;
            }
            function_ids.push(func_id);
            max_id = max_id.max(func_id.id as usize);
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
            functions.push(Arc::new(threaded));
        }

        // assemble table
        Self {
            functions,
            index_by_id,
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
    pub(super) fn get_by_index(&self, index: u32) -> Option<&Arc<ThreadedFunction>> {
        self.functions.get(index as usize)
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

        // pre-thread all functions for fast dispatch
        let threaded_functions = ThreadedFunctionTable::new(&tree);

        Self {
            tree,
            strings,
            managed_heap,
            raw_heap: RawHeap::new(),
            globals,
            externals: HashMap::new(),
            options,
            threaded_functions,
            call_stack: Vec::new(),
            value_stack: Vec::new(),
            local_stack: Vec::new(),
            statistics: Statistics::new(),
        }
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
    pub fn collect_garbage(&mut self) {
        // collect roots from all frames
        let mut roots = Vec::new();
        for frame in &self.call_stack {
            frame.collect_roots(&self.value_stack, &self.local_stack, &mut roots);
        }

        self.managed_heap.collect(&roots);
    }
}
