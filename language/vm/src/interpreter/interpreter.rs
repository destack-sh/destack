use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "stats")]
use std::time::Duration;

use destack_base::ImmutableStringPool;
use destack_mir as mir;

use crate::diagnostic::{DiagnosticAnchor, Error, FrameInfo, RuntimeError};
use crate::memory::{
    HeapHandle, ManagedHeap, RawCellStorage, RawHeap, RawPointer, STRING_FLAG_IS_ASCII,
    STRING_FLAG_IS_INTERNED, STRING_FLAG_IS_STATIC, StringLayout, Value,
};

use super::decode::thread_function;
#[cfg(feature = "stats")]
use super::statistics::InstructionProfile;
use super::threaded::{
    ConstValue, CopyRange, INVALID_FUNCTION_INDEX, ThreadedFunction, ThreadedInstructionData,
};
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
    /// Interned string literals mapped to heap handles.
    pub(super) string_literals: HashMap<String, HeapHandle>,
    /// Raw heap buffers for string payloads.
    pub(super) string_buffers: HashMap<HeapHandle, RawPointer>,
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
            .field(
                "string_literals",
                &format!("<{} literals>", self.string_literals.len()),
            )
            .field(
                "string_buffers",
                &format!("<{} buffers>", self.string_buffers.len()),
            )
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
        let function_name_map = Self::build_function_name_map(&tree, &strings);

        // pre-thread all functions for fast dispatch
        let threaded_functions = ThreadedFunctionTable::new(&tree);

        // assign a unique interpreter id
        let id = INTERPRETER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        let mut interpreter = Self {
            id,
            tree,
            strings,
            managed_heap: ManagedHeap::new(),
            raw_heap: RawHeap::new(),
            string_literals: HashMap::new(),
            string_buffers: HashMap::new(),
            globals: GlobalStorage::new(),
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
        };

        // initialize global storage after heap setup
        interpreter.globals = interpreter.initialize_globals();

        // pre-intern string literals for threaded const instructions
        interpreter.pre_intern_threaded_strings();

        interpreter
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
    fn initialize_globals(&mut self) -> GlobalStorage {
        // seed empty global storage
        let mut globals = GlobalStorage::new();

        // snapshot globals to avoid borrowing self during initialization
        let global_entries: Vec<_> = self
            .tree
            .iter_nodes::<mir::Global>()
            .map(|(id, global)| {
                (
                    id,
                    global.ty,
                    global.is_import(),
                    global.initializer.clone(),
                )
            })
            .collect();

        // populate globals from initializers
        for (id, ty, is_import, initializer) in global_entries {
            // skip imported globals
            if is_import {
                continue;
            }

            // convert initializer when present
            let value = match initializer.as_ref() {
                Some(init) => self.convert_initializer(init, ty),
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
        &mut self,
        init: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Value {
        // select conversion strategy
        match init {
            mir::GlobalInitializer::Zero => self.zero_value(ty),
            mir::GlobalInitializer::Scalar(constant) => self.constant_to_value(constant),
            mir::GlobalInitializer::Bytes(bytes) => {
                // convert bytes to u8 values
                let values: Vec<Value> = bytes.iter().map(|&b| Value::uint(b as u64, 8)).collect();

                // allocate managed aggregate for bytes
                let handle = self.managed_heap.allocate_with_values(values);

                Value::aggregate(handle)
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                // convert each element recursively
                let values: Vec<Value> = elements
                    .iter()
                    .map(|e| self.convert_initializer(e, ty))
                    .collect();

                // allocate managed aggregate for elements
                let handle = self.managed_heap.allocate_with_values(values);

                Value::aggregate(handle)
            }
        }
    }

    /// Convert a MIR constant to a runtime value.
    fn constant_to_value(&mut self, constant: &mir::Constant) -> Value {
        // route string constants through the literal interner
        if let mir::Constant::String { value } = constant {
            return self.intern_string_literal(value);
        }

        // fall back to non-string conversion
        Value::from(constant)
    }

    /// Create a zero value for a given type.
    fn zero_value(&mut self, ty: mir::LocalNodeId<mir::Type>) -> Value {
        // resolve the type node
        let ty_node = self.tree.get(ty).clone();

        // build a zero value based on type
        match ty_node {
            mir::Type::Int { width, signed } => {
                // select signed or unsigned zero
                if signed {
                    Value::int(0, width as u8)
                } else {
                    Value::uint(0, width as u8)
                }
            }
            mir::Type::Float { width } => {
                // select float width
                if width == 32 {
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
                // recursively initialize tuple elements
                let values: Vec<Value> = elements.into_iter().map(|e| self.zero_value(e)).collect();

                // allocate managed aggregate for tuple
                let handle = self.managed_heap.allocate_with_values(values);

                Value::aggregate(handle)
            }
            mir::Type::Array {
                element,
                length,
                copyability: _,
            } => {
                // build an array of repeated element zeros
                let elem_zero = self.zero_value(element);
                let values: Vec<Value> = (0..length).map(|_| elem_zero).collect();

                // allocate managed aggregate for array
                let handle = self.managed_heap.allocate_with_values(values);

                Value::aggregate(handle)
            }
            // for other types, use void
            _ => Value::VOID,
        }
    }

    /// Intern a string literal and return its managed value.
    pub(super) fn intern_string_literal(&mut self, value: &str) -> Value {
        // reuse existing interned handle
        if let Some(handle) = self.string_literals.get(value).copied() {
            return Value::string(handle);
        }

        // compute UTF-8 byte length
        let length_bytes = value.len();

        // compute UTF-16 code unit length
        let length_utf16 = value.encode_utf16().count();

        // validate length bounds for string metadata
        if length_bytes > u32::MAX as usize || length_utf16 > u32::MAX as usize {
            panic!("string literal exceeds u32 length limits");
        }

        // materialize length fields
        let length_bytes = length_bytes as u32;
        let length_utf16 = length_utf16 as u32;

        // compute string flags for literal storage
        let mut flags = STRING_FLAG_IS_INTERNED | STRING_FLAG_IS_STATIC;
        if value.is_ascii() {
            flags |= STRING_FLAG_IS_ASCII;
        }

        // allocate raw UTF-8 payload
        let data = self.allocate_string_bytes(value.as_bytes());

        // allocate the managed string header
        let handle =
            self.allocate_string_cell(length_utf16, length_bytes, 0, length_bytes, flags, data);

        // record interned handle and payload buffer
        self.string_literals.insert(value.to_string(), handle);
        if !data.is_null() {
            self.string_buffers.insert(handle, data);
        }

        Value::string(handle)
    }

    /// Pre-intern string literals referenced by threaded const instructions.
    fn pre_intern_threaded_strings(&mut self) {
        // collect string literals without holding a mutable borrow
        let mut literals = Vec::new();
        for function in &self.threaded_functions.functions {
            for block in &function.blocks {
                for instruction in &block.instructions {
                    let ThreadedInstructionData::Const { value, .. } = &instruction.data else {
                        continue;
                    };
                    let ConstValue::String(literal) = value else {
                        continue;
                    };
                    literals.push(literal.clone());
                }
            }
        }

        // intern collected literals
        for literal in literals {
            self.intern_string_literal(&literal);
        }
    }

    /// Allocate raw heap storage for string payload bytes.
    fn allocate_string_bytes(&mut self, bytes: &[u8]) -> RawPointer {
        // treat empty payloads as null pointers
        if bytes.is_empty() {
            return RawPointer::NULL;
        }

        // allocate raw heap buffer for payload
        self.raw_heap.allocate_with_bytes(bytes)
    }

    /// Allocate a managed string header cell.
    fn allocate_string_cell(
        &mut self,
        length_utf16: u32,
        length_bytes: u32,
        hash: u64,
        capacity: u32,
        flags: u32,
        data: RawPointer,
    ) -> HeapHandle {
        // assemble header slots for the string layout
        let mut slots = vec![Value::VOID; StringLayout::SLOT_COUNT];
        slots[StringLayout::LENGTH_UTF16] = Value::uint(length_utf16 as u64, 32);
        slots[StringLayout::LENGTH_BYTES] = Value::uint(length_bytes as u64, 32);
        slots[StringLayout::HASH] = Value::uint(hash, 64);
        slots[StringLayout::CAPACITY] = Value::uint(capacity as u64, 32);
        slots[StringLayout::FLAGS] = Value::uint(flags as u64, 32);
        slots[StringLayout::DATA] = Value::raw_pointer(data);

        // allocate managed heap cell for the header
        self.managed_heap.allocate_with_values(slots)
    }

    /// Get the slot count for a raw pointer.
    pub(super) fn raw_slot_count(&self, pointer: RawPointer) -> Option<usize> {
        Some(self.raw_heap.get(pointer)?.storage.len())
    }

    /// Read a raw slot, dispatching to the correct raw heap.
    pub(super) fn read_raw_slot(
        &self,
        pointer: RawPointer,
        slot_index: usize,
        bounds_checks: bool,
    ) -> Result<Value, Error> {
        let cell = self.raw_heap.get(pointer).ok_or(Error::InvalidHeapHandle)?;

        match &cell.storage {
            RawCellStorage::Bytes(bytes) => {
                // treat empty slot 0 as void
                if bytes.is_empty() && slot_index == 0 {
                    return Ok(Value::VOID);
                }

                // enforce bounds even in unchecked mode to avoid UB
                if slot_index >= bytes.len() {
                    return Err(Error::InvalidFieldAccess {
                        index: slot_index as u32,
                        field_count: bytes.len(),
                    });
                }

                let byte = bytes[slot_index];
                Ok(Value::uint(byte as u64, 8))
            }
            RawCellStorage::Values(slots) => {
                // treat empty slot 0 as void
                if slots.is_empty() && slot_index == 0 {
                    return Ok(Value::VOID);
                }

                // fast path without bounds checks
                if !bounds_checks {
                    debug_assert!(slot_index < slots.len(), "raw slot out of bounds");
                    // #Safety: bounds checks are disabled and slot is trusted
                    let value = unsafe { *slots.get_unchecked(slot_index) };
                    return Ok(value);
                }

                // read the slot when in bounds
                if let Some(value) = slots.get(slot_index).copied() {
                    return Ok(value);
                }

                Err(Error::InvalidFieldAccess {
                    index: slot_index as u32,
                    field_count: slots.len(),
                })
            }
        }
    }

    /// Write a raw slot, dispatching to the correct raw heap.
    pub(super) fn write_raw_slot(
        &mut self,
        pointer: RawPointer,
        slot_index: usize,
        value: Value,
        bounds_checks: bool,
    ) -> Result<(), Error> {
        let cell = self
            .raw_heap
            .get_mut(pointer)
            .ok_or(Error::InvalidHeapHandle)?;

        match &mut cell.storage {
            RawCellStorage::Bytes(bytes) => {
                let raw = value.as_uint().ok_or_else(|| Error::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: format!("{value:?}"),
                })?;
                let byte = raw as u8;

                // enforce bounds even in unchecked mode to avoid UB
                if slot_index >= bytes.len() {
                    return Err(Error::InvalidFieldAccess {
                        index: slot_index as u32,
                        field_count: bytes.len(),
                    });
                }

                bytes[slot_index] = byte;
                Ok(())
            }
            RawCellStorage::Values(slots) => {
                // resize slots as needed when bounds checks are enabled
                if bounds_checks && slots.len() <= slot_index {
                    slots.resize(slot_index + 1, Value::VOID);
                }

                // fast path without bounds checks
                if !bounds_checks {
                    debug_assert!(slot_index < slots.len(), "raw slot out of bounds");
                    // #Safety: bounds checks are disabled and slot is trusted
                    unsafe {
                        *slots.get_unchecked_mut(slot_index) = value;
                    }
                    return Ok(());
                }

                // write the slot when in bounds
                if let Some(slot) = slots.get_mut(slot_index) {
                    *slot = value;
                    return Ok(());
                }

                Err(Error::InvalidFieldAccess {
                    index: slot_index as u32,
                    field_count: slots.len(),
                })
            }
        }
    }

    /// Sweep raw string payloads for freed managed string headers.
    fn sweep_string_buffers(&mut self) {
        // collect handles to free without mutating during iteration
        let mut freed_buffers = Vec::new();
        for (&handle, &raw_ptr) in &self.string_buffers {
            if !self.managed_heap.is_allocated(handle) {
                freed_buffers.push((handle, raw_ptr));
            }
        }

        // release raw payloads for freed strings
        for (handle, raw_ptr) in freed_buffers {
            self.string_buffers.remove(&handle);
            if !raw_ptr.is_null() {
                self.raw_heap.free(raw_ptr);
            }
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
            .ok_or_else(|| Error::TypeMismatch {
                expected: "raw_pointer".to_string(),
                actual: format!("{data_value:?}"),
            })?;
        if data_ptr.is_null() {
            return Err(Error::NullPointerDereference);
        }

        let raw_cell = self
            .raw_heap
            .get(data_ptr)
            .ok_or(Error::InvalidHeapHandle)?;
        let bytes = match &raw_cell.storage {
            RawCellStorage::Bytes(bytes) => bytes,
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "byte buffer".to_string(),
                    actual: format!("{data_value:?}"),
                });
            }
        };

        if length > bytes.len() {
            return Err(Error::InvalidHeapHandle);
        }

        let slice = &bytes[..length];
        String::from_utf8(slice.to_vec()).map_err(|_| Error::InvalidCast)
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

        // collect roots from globals
        for value in self.globals.values() {
            if let Some(handle) = value.as_heap_handle() {
                roots.push(handle);
            }
        }

        // collect roots from interned string literals
        for handle in self.string_literals.values() {
            roots.push(*handle);
        }

        // run collection
        let freed_cells = self.managed_heap.collect(&roots);

        // sweep raw payload buffers for freed strings
        self.sweep_string_buffers();
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
