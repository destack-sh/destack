use std::ptr::NonNull;
#[cfg(feature = "stats")]
use std::time::Duration;

use destack_mir as mir;

use crate::diagnostic::{DiagnosticAnchor, Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::execute::Continuation;
use crate::isolate::{ExternalFnPtr, GlobalStorage, IsolateState};
use crate::memory::{GcStats, HeapHandle, RawCellStorage, RawPointer, ReferenceMeta, Value};
use crate::telemetry::Statistics;

use super::super::decode::{
    ConstValue, INVALID_FUNCTION_INDEX, ThreadedFunction, ThreadedInstructionData, thread_function,
};
use super::Frame;
#[cfg(feature = "stats")]
use crate::telemetry::InstructionProfile;

/// Interpreter engine state for threaded execution.
#[derive(Debug)]
pub(crate) struct InterpreterState {
    /// Pre-threaded functions for fast dispatch.
    pub(crate) threaded_functions: ThreadedFunctionTable,
    /// Explicit call stack (used for GC roots and error reporting).
    pub(crate) call_stack: Vec<Frame>,
    /// SSA value stack for all active frames.
    pub(crate) value_stack: Vec<Value>,
    /// Local variable stack for all active frames.
    pub(crate) local_stack: Vec<Value>,
    /// Execution statistics.
    pub(crate) statistics: Statistics,
    /// Optional instruction profiling sampler.
    #[cfg(feature = "stats")]
    pub(crate) instruction_profile: Option<InstructionProfile>,
}

/// Interpreter execution engine for threaded dispatch.
#[derive(Debug)]
pub struct InterpreterEngine {
    /// Interpreter engine state for execution.
    pub(crate) state: InterpreterState,
}

impl InterpreterEngine {
    /// Create a new interpreter engine for the given isolate state.
    pub(crate) fn new(isolate: &IsolateState) -> Self {
        let threaded_functions = ThreadedFunctionTable::new(&isolate.tree);

        Self {
            state: InterpreterState {
                threaded_functions,
                call_stack: Vec::new(),
                value_stack: Vec::new(),
                local_stack: Vec::new(),
                statistics: Statistics::new(),
                #[cfg(feature = "stats")]
                instruction_profile: None,
            },
        }
    }

    /// Borrow a context with access to isolate and engine state.
    pub(crate) fn context<'a>(
        &'a mut self,
        isolate: &'a mut IsolateState,
    ) -> InterpreterContext<'a> {
        InterpreterContext {
            isolate,
            engine: &mut self.state,
        }
    }

    /// Get the current call stack for this engine.
    pub(crate) fn call_stack(&self) -> &[Frame] {
        &self.state.call_stack
    }
}

/// Interpreter context with access to isolate and engine state.
pub(crate) struct InterpreterContext<'a> {
    /// Shared isolate state for this execution.
    pub(crate) isolate: &'a mut IsolateState,
    /// Interpreter engine state for execution.
    pub(crate) engine: &'a mut InterpreterState,
}

/// Threaded function registry for fast lookup.
#[derive(Debug)]
pub(crate) struct ThreadedFunctionTable {
    /// Threaded functions by dense index.
    functions: Vec<ThreadedFunction>,
    /// Mapping from function id to threaded index (INVALID_FUNCTION_INDEX if missing).
    index_by_id: Vec<u32>,
    /// Import status by function id.
    /// NOTE #Cleanup: do we really need ThreadedFunctionTable::is_import_by_id?
    is_import_by_id: Vec<bool>,
}

impl ThreadedFunctionTable {
    /// Build a threaded function table for the MIR tree.
    pub(crate) fn new(tree: &mir::NodeTree) -> Self {
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
    pub(crate) fn index_for(&self, func_id: mir::LocalNodeId<mir::Function>) -> Option<u32> {
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
    pub(crate) fn get_by_index(&self, index: u32) -> Option<&ThreadedFunction> {
        self.functions.get(index as usize)
    }

    /// Get a threaded function pointer by index.
    pub(crate) fn get_ptr_by_index(&self, index: u32) -> Option<NonNull<ThreadedFunction>> {
        self.functions.get(index as usize).map(NonNull::from)
    }

    /// Report whether a function id references an import.
    pub(crate) fn is_import(&self, func_id: mir::LocalNodeId<mir::Function>) -> bool {
        self.is_import_by_id
            .get(func_id.id as usize)
            .copied()
            .unwrap_or(false)
    }
}

#[allow(dead_code)]
impl<'a> InterpreterContext<'a> {
    /// Enable instruction profiling with the given sampling interval.
    #[cfg(feature = "stats")]
    pub(crate) fn enable_instruction_profile(&mut self, sample_interval: Duration) {
        self.engine.instruction_profile = Some(InstructionProfile::new(sample_interval));
    }

    /// Reset instruction profiling samples without disabling sampling.
    #[cfg(feature = "stats")]
    pub(crate) fn reset_instruction_profile(&mut self) {
        if let Some(profile) = self.engine.instruction_profile.as_mut() {
            profile.reset();
        }
    }

    /// Clear instruction profiling data and disable sampling.
    #[cfg(feature = "stats")]
    pub(crate) fn clear_instruction_profile(&mut self) {
        self.engine.instruction_profile = None;
    }

    /// Return a compact instruction profile report if available.
    #[cfg(feature = "stats")]
    pub(crate) fn instruction_profile_report(&self, target_percent: f64) -> Option<String> {
        self.engine
            .instruction_profile
            .as_ref()
            .map(|profile| profile.summary_target(target_percent).format_compact())
    }

    /// Initialize global variables from the MIR tree.
    pub(crate) fn initialize_globals(&mut self) -> RuntimeResult<()> {
        // seed empty global storage
        let mut globals = GlobalStorage::new();

        // snapshot globals to avoid borrowing self during initialization
        let global_entries: Vec<_> = self
            .isolate
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
                Some(init) => self.convert_initializer(init, ty)?,
                None => Value::VOID,
            };
            globals.set(id, value);
        }

        // store initialized globals
        self.isolate.globals = globals;

        Ok(())
    }

    /// Convert a global initializer to a runtime value.
    fn convert_initializer(
        &mut self,
        init: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        // select conversion strategy
        match init {
            mir::GlobalInitializer::Zero => self.zero_value(ty),
            mir::GlobalInitializer::Scalar(constant) => Ok(self.constant_to_value(constant)),
            mir::GlobalInitializer::Bytes(bytes) => {
                // convert bytes to u8 values
                let values: Vec<Value> = bytes.iter().map(|&b| Value::uint(b as u64, 8)).collect();

                // allocate managed aggregate for bytes
                let handle = self.isolate.managed_heap.allocate_with_values(values);

                Ok(Value::aggregate(handle))
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                // convert each element recursively
                let values: Vec<Value> = elements
                    .iter()
                    .map(|e| self.convert_initializer(e, ty))
                    .collect::<RuntimeResult<_>>()?;

                // allocate managed aggregate for elements
                let handle = self.isolate.managed_heap.allocate_with_values(values);

                Ok(Value::aggregate(handle))
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
    fn zero_value(&mut self, ty: mir::LocalNodeId<mir::Type>) -> RuntimeResult<Value> {
        // resolve the type node
        let ty_node = self.isolate.tree.get(ty).clone();

        // build a zero value based on type
        match ty_node {
            mir::Type::Void => Ok(Value::VOID),
            mir::Type::Int { width, signed } => {
                // select signed or unsigned zero
                if signed {
                    Ok(Value::int(0, width as u8))
                } else {
                    Ok(Value::uint(0, width as u8))
                }
            }
            mir::Type::Isize => Ok(Value::int(0, usize::BITS as u8)),
            mir::Type::Usize => Ok(Value::uint(0, usize::BITS as u8)),
            mir::Type::Float { width } => {
                // select float width
                if width == 32 {
                    Ok(Value::float32(0.0))
                } else {
                    Ok(Value::float64(0.0))
                }
            }
            mir::Type::Boolean => Ok(Value::bool(false)),
            mir::Type::Type => Err(self.make_error(Error::UnsupportedZeroValue {
                ty: format!("{ty_node:?}"),
            })),
            mir::Type::Reference {
                kind,
                address_space,
                mutability,
                is_nullable,
                ..
            } => {
                if !is_nullable {
                    return Err(self.make_error(Error::UnsupportedZeroValue {
                        ty: format!("{ty_node:?}"),
                    }));
                }

                let meta = ReferenceMeta::new(kind, address_space, mutability, is_nullable);
                match kind {
                    mir::ReferenceKind::Managed => {
                        Ok(Value::managed_reference_with_meta(HeapHandle::NULL, meta))
                    }
                    mir::ReferenceKind::Owned
                    | mir::ReferenceKind::Borrowed
                    | mir::ReferenceKind::Raw => {
                        Ok(Value::raw_pointer_with_meta(RawPointer::NULL, meta))
                    }
                }
            }
            mir::Type::Tuple {
                elements,
                copyability: _,
            } => {
                // recursively initialize tuple elements
                let values: Vec<Value> = elements
                    .into_iter()
                    .map(|e| self.zero_value(e))
                    .collect::<RuntimeResult<_>>()?;

                // allocate managed aggregate for tuple
                let handle = self.isolate.managed_heap.allocate_with_values(values);

                Ok(Value::aggregate(handle))
            }
            mir::Type::Array {
                element,
                length,
                copyability: _,
            } => {
                // build an array of repeated element zeros
                let elem_zero = self.zero_value(element)?;
                let values: Vec<Value> = (0..length).map(|_| elem_zero).collect();

                // allocate managed aggregate for array
                let handle = self.isolate.managed_heap.allocate_with_values(values);

                Ok(Value::aggregate(handle))
            }
            _ => Err(self.make_error(Error::UnsupportedZeroValue {
                ty: format!("{ty_node:?}"),
            })),
        }
    }

    /// Intern a string literal and return its managed value.
    pub(crate) fn intern_string_literal(&mut self, value: &str) -> Value {
        self.isolate.intern_string_literal(value)
    }

    /// Pre-intern string literals referenced by threaded const instructions.
    pub(crate) fn pre_intern_threaded_strings(&mut self) {
        // collect string literals without holding a mutable borrow
        let mut literals = Vec::new();
        for function in &self.engine.threaded_functions.functions {
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

    /// Get the slot count for a raw pointer.
    pub(crate) fn raw_slot_count(&self, pointer: RawPointer) -> Option<usize> {
        Some(self.isolate.raw_heap.get(pointer)?.storage.len())
    }

    /// Read a raw slot, dispatching to the correct raw heap.
    pub(crate) fn read_raw_slot(
        &self,
        pointer: RawPointer,
        slot_index: usize,
        bounds_checks: bool,
    ) -> Result<Value, Error> {
        let cell = self
            .isolate
            .raw_heap
            .get(pointer)
            .ok_or(Error::InvalidHeapHandle)?;

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
    pub(crate) fn write_raw_slot(
        &mut self,
        pointer: RawPointer,
        slot_index: usize,
        value: Value,
        bounds_checks: bool,
    ) -> Result<(), Error> {
        let cell = self
            .isolate
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
        // delegate to the isolate string interner
        self.isolate.sweep_string_buffers();
    }

    /// Resolve an external handler for an imported function id.
    pub(crate) fn external_for_id(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Result<ExternalFnPtr, RuntimeError> {
        let index = function_id.id as usize;
        if let Some(handler) = self.isolate.externals_by_id.get(index).copied().flatten() {
            return Ok(handler);
        }

        if self.isolate.externals_by_id.len() <= index {
            self.isolate.externals_by_id.resize(index + 1, None);
        }

        let func = self.isolate.tree.get(function_id);
        let name = self.isolate.strings.get(func.name).to_string();
        let handler = self
            .isolate
            .externals
            .get(&name)
            .ok_or_else(|| self.make_error(Error::ExternalFunctionNotFound { name }))?;
        let handler_ptr = NonNull::from(handler.as_ref());
        self.isolate.externals_by_id[index] = Some(handler_ptr);

        Ok(handler_ptr)
    }

    /// Create an error with current call stack.
    #[cold]
    pub(crate) fn make_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info())
    }

    /// Allocate an aggregate on the heap and return it as a Value.
    pub(crate) fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        self.isolate.allocate_aggregate(values)
    }

    /// Allocate a 2-element aggregate on the heap (avoids Vec allocation).
    #[inline]
    pub(crate) fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        self.isolate.allocate_pair(first, second)
    }

    /// Allocate a 1-element aggregate on the heap (avoids Vec allocation).
    #[inline]
    pub(crate) fn allocate_single(&mut self, value: Value) -> Value {
        self.isolate.allocate_single(value)
    }

    /// Get the slots of an aggregate value (looking up from heap if needed).
    pub(crate) fn get_aggregate_slots(&self, value: &Value) -> Option<&[Value]> {
        value
            .as_heap_handle()
            .and_then(|handle| self.isolate.managed_heap.get(handle))
            .map(|cell| cell.slots.as_slice())
    }

    /// Create an error with instruction anchor.
    #[cold]
    pub(crate) fn make_error_at(
        &self,
        error: Error,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> RuntimeError {
        let frame = self.engine.call_stack.last();
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
        self.engine
            .call_stack
            .iter()
            .map(|f| {
                let func = self.isolate.tree.get(f.function);
                let name = self.isolate.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Run garbage collection on the managed heap.
    pub(crate) fn collect_garbage(&mut self) -> GcStats {
        self.collect_garbage_with_continuations(&[])
    }

    /// Run garbage collection including suspended continuations.
    pub(crate) fn collect_garbage_with_continuations(
        &mut self,
        continuations: &[Continuation],
    ) -> GcStats {
        // collect roots from active frames
        let mut roots = Vec::new();
        for frame in &self.engine.call_stack {
            frame.collect_roots(
                &self.engine.value_stack,
                &self.engine.local_stack,
                &mut roots,
            );
        }

        // collect roots from continuations
        for continuation in continuations {
            continuation.collect_roots(&mut roots);
        }

        // collect roots from globals
        for value in self.isolate.globals.values() {
            if let Some(handle) = value.as_heap_handle() {
                roots.push(handle);
            }
        }

        // collect roots from interned string literals
        self.isolate.collect_string_roots(&mut roots);

        // run collection
        let freed_cells = self.isolate.managed_heap.collect(&roots);

        // sweep raw payload buffers for freed strings
        self.sweep_string_buffers();
        let live_cells = self.isolate.managed_heap.cell_count();

        GcStats {
            freed_cells,
            live_cells,
        }
    }
}
