use std::collections::HashMap;
use std::sync::Arc;

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use crate::diagnostic::{DiagnosticAnchor, Error, FrameInfo, RuntimeError};
use crate::memory::{ManagedHeap, RawHeap, Value};

use super::decode::thread_function;
use super::threaded::ThreadedFunction;
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
    /// Pre-threaded functions for fast dispatch (Arc for cheap cloning).
    pub(super) threaded_functions: HashMap<mir::LocalNodeId<mir::Function>, Arc<ThreadedFunction>>,
    /// Explicit call stack (used for GC roots and error reporting).
    pub(super) call_stack: Vec<Frame>,
    /// Execution statistics.
    pub statistics: Statistics,
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

        // Pre-thread all functions for fast dispatch
        let threaded_functions = Self::thread_all_functions(&tree);

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
            statistics: Statistics::new(),
        }
    }

    /// Thread all functions in the MIR tree for fast dispatch.
    fn thread_all_functions(
        tree: &mir::NodeTree,
    ) -> HashMap<mir::LocalNodeId<mir::Function>, Arc<ThreadedFunction>> {
        let mut threaded = HashMap::new();

        for (func_id, _) in tree.iter_nodes::<mir::Function>() {
            if let Some(tf) = thread_function(tree, func_id) {
                threaded.insert(func_id, Arc::new(tf));
            }
        }

        threaded
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
            mir::Type::Tuple { elements } => {
                let values: Vec<Value> = elements
                    .iter()
                    .map(|e| Self::zero_value(tree, heap, *e))
                    .collect();
                let handle = heap.allocate_with_values(values);
                Value::aggregate(handle)
            }
            mir::Type::Array { element, length } => {
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
    pub(super) fn make_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info())
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        let handle = self.managed_heap.allocate_with_values(values);
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
            frame.collect_roots(&mut roots);
        }

        self.managed_heap.collect(&roots);
    }
}
