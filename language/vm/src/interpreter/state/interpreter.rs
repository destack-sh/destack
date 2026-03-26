use std::collections::HashMap;
use std::ops::Deref;
use std::ptr::NonNull;
#[cfg(feature = "stats")]
use std::time::Duration;

use destack_mir as mir;

use crate::diagnostic::{Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::executable::{Executable, FunctionTable};
use crate::execute::Continuation;
use crate::isolate::{ExternalFn, ExternalFnPtr, GlobalStorage, StringInterner};
use crate::snapshot::InterpreterImage;
use crate::telemetry::Statistics;
use destack_heap::{
    GcStats, ManagedReference, MemoryContext, RawPointer, ReferenceMeta, SharedPointer, Value,
    string_layout_matches,
};

use super::Frame;
#[cfg(feature = "stats")]
use crate::telemetry::InstructionProfile;

/// Interpreter execution engine.
#[derive(Debug)]
pub struct Interpreter {
    /// Explicit call stack used for execution and root walking.
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

impl Interpreter {
    /// Create a new interpreter engine.
    pub(crate) fn new() -> Self {
        Self {
            call_stack: Vec::new(),
            value_stack: Vec::new(),
            local_stack: Vec::new(),
            statistics: Statistics::new(),
            #[cfg(feature = "stats")]
            instruction_profile: None,
        }
    }

    /// Get the current call stack for this engine.
    pub(crate) fn call_stack(&self) -> &[Frame] {
        &self.call_stack
    }

    /// Capture one immutable interpreter image.
    pub(crate) fn image(&self) -> InterpreterImage {
        let call_stack = self.call_stack.iter().map(Frame::image).collect();

        InterpreterImage {
            call_stack,
            value_stack: self.value_stack.clone(),
            local_stack: self.local_stack.clone(),
            statistics: self.statistics.clone(),
        }
    }

    /// Create one interpreter from an immutable image.
    pub(crate) fn from_image(
        functions: &FunctionTable,
        image: &InterpreterImage,
    ) -> RuntimeResult<Self> {
        let mut interpreter = Self::new();

        // restore the mutable execution state
        let call_stack = image
            .call_stack
            .iter()
            .map(|frame| Frame::from_image(frame, functions))
            .collect::<RuntimeResult<Vec<_>>>()?;

        interpreter.call_stack = call_stack;
        interpreter.value_stack = image.value_stack.clone();
        interpreter.local_stack = image.local_stack.clone();
        interpreter.statistics = image.statistics.clone();

        #[cfg(feature = "stats")]
        {
            interpreter.instruction_profile = None;
        }

        Ok(interpreter)
    }

    /// Enable instruction profiling with the given sampling interval.
    #[cfg(feature = "stats")]
    pub(crate) fn enable_instruction_profile(&mut self, sample_interval: Duration) {
        self.instruction_profile = Some(InstructionProfile::new(sample_interval));
    }

    /// Reset instruction profiling samples without disabling sampling.
    #[cfg(feature = "stats")]
    pub(crate) fn reset_instruction_profile(&mut self) {
        if let Some(profile) = self.instruction_profile.as_mut() {
            profile.reset();
        }
    }

    /// Clear instruction profiling data and disable sampling.
    #[cfg(feature = "stats")]
    pub(crate) fn clear_instruction_profile(&mut self) {
        self.instruction_profile = None;
    }

    /// Return a compact instruction profile report if available.
    #[cfg(feature = "stats")]
    pub(crate) fn instruction_profile_report(&self, target_percent: f64) -> Option<String> {
        self.instruction_profile
            .as_ref()
            .map(|profile| profile.summary_target(target_percent).format_compact())
    }

    /// Resolve a function id by name from the executable.
    pub(crate) fn lookup_function_id(
        executable: &Executable,
        name: &str,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        executable.function_id_by_name.get(name).copied()
    }

    /// Borrow the lowered function table.
    pub(crate) fn functions(executable: &Executable) -> &FunctionTable {
        &executable.functions
    }

    /// Resolve a vtable id for a vtable global.
    pub(crate) fn vtable_for_global(
        executable: &Executable,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<mir::VtableId> {
        executable.vtable_id_by_global.get(&global).copied()
    }

    /// Resolve an external handler for an imported function id.
    pub(crate) fn external_for_id(
        &self,
        executable: &Executable,
        externals: &HashMap<String, ExternalFn>,
        externals_by_id: &mut Vec<Option<ExternalFnPtr>>,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Result<ExternalFnPtr, RuntimeError> {
        let index = function_id.id as usize;
        if let Some(handler) = externals_by_id.get(index).copied().flatten() {
            return Ok(handler);
        }

        if externals_by_id.len() <= index {
            externals_by_id.resize(index + 1, None);
        }

        let func = executable.tree.get(function_id);
        let name = executable.strings.get(func.name).to_string();
        let handler = externals
            .get(&name)
            .ok_or_else(|| self.make_error(executable, Error::ExternalFunctionNotFound { name }))?;
        let handler_ptr = NonNull::from(handler.as_ref());
        externals_by_id[index] = Some(handler_ptr);

        Ok(handler_ptr)
    }

    /// Create an error with current call stack.
    #[cold]
    pub(crate) fn make_error(&self, executable: &Executable, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info(executable))
    }
}

/// Aggregate slots backed by a heap read guard.
pub(crate) struct AggregateSlots<'a> {
    /// Borrowed aggregate slot slice.
    borrowed: Option<&'a [Value]>,
    /// Owned aggregate slot storage.
    owned: Option<Vec<Value>>,
}

impl<'a> AggregateSlots<'a> {
    /// Create aggregate slots from one owned slot list.
    pub(crate) fn owned(slots: Vec<Value>) -> Self {
        Self {
            borrowed: None,
            owned: Some(slots),
        }
    }
}

impl<'a> Deref for AggregateSlots<'a> {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        if let Some(slots) = self.borrowed {
            return slots;
        }

        self.owned
            .as_deref()
            .expect("aggregate slots should always have one backing")
    }
}

impl Interpreter {
    /// Initialize global variables from the MIR tree.
    pub(crate) fn initialize_globals(
        &mut self,
        executable: &Executable,
        string_interner: &mut StringInterner,
        globals: &mut GlobalStorage,
        mut memory: MemoryContext<'_>,
    ) -> RuntimeResult<()> {
        // seed empty global storage
        let mut initialized_globals = GlobalStorage::new();

        // snapshot globals to avoid borrowing executable during initialization
        let global_entries: Vec<_> = executable
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
                Some(init) => self.convert_initializer(
                    executable,
                    string_interner,
                    memory.reborrow(),
                    init,
                    ty,
                )?,
                None => Value::VOID,
            };
            initialized_globals.set(id, value);
        }

        // store initialized globals
        *globals = initialized_globals;

        Ok(())
    }

    /// Convert a global initializer to a runtime value.
    fn convert_initializer(
        &mut self,
        executable: &Executable,
        string_interner: &mut StringInterner,
        mut memory: MemoryContext<'_>,
        init: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        // select conversion strategy
        match init {
            mir::GlobalInitializer::Zero => self.zero_value(executable, memory.reborrow(), ty),
            mir::GlobalInitializer::Scalar(constant) => Ok(self.constant_to_value(constant)),
            mir::GlobalInitializer::String(value) => {
                // validate the declared string layout
                self.validate_string_initializer_type(executable, ty)?;

                let heap = memory.heap();
                string_interner
                    .try_intern_string_literal(heap, value)
                    .map_err(|error| self.make_error(executable, error))
            }
            mir::GlobalInitializer::Bytes(bytes) => {
                // convert bytes to u8 values
                let values: Vec<Value> = bytes.iter().map(|&b| Value::uint(b as u64, 8)).collect();

                // allocate managed aggregate for bytes
                let handle = memory
                    .heap()
                    .allocate_packed_values(values)
                    .map_err(|error| self.make_error(executable, Error::from(error)))?;

                Ok(Value::aggregate(handle))
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                // convert each element recursively
                let values: Vec<Value> = elements
                    .iter()
                    .map(|e| {
                        self.convert_initializer(
                            executable,
                            string_interner,
                            memory.reborrow(),
                            e,
                            ty,
                        )
                    })
                    .collect::<RuntimeResult<_>>()?;

                // allocate managed aggregate for elements
                let handle = memory
                    .heap()
                    .allocate_packed_values(values)
                    .map_err(|error| self.make_error(executable, Error::from(error)))?;

                Ok(Value::aggregate(handle))
            }
        }
    }

    /// Validate that a string initializer matches the expected layout.
    fn validate_string_initializer_type(
        &self,
        executable: &Executable,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<()> {
        // resolve the reference type
        let mir::Type::Reference { kind, pointee, .. } = executable.tree.get(ty) else {
            return Err(self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "ref<managed String>".to_string(),
                    actual: format!("{:?}", executable.tree.get(ty)),
                },
            ));
        };

        // ensure the string is a managed reference
        if *kind != mir::ReferenceKind::Managed {
            return Err(self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "ref<managed String>".to_string(),
                    actual: format!("{:?}", executable.tree.get(ty)),
                },
            ));
        }

        // validate the struct layout matches the runtime definition
        if !string_layout_matches(&executable.tree, *pointee) {
            return Err(self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "ref<managed String>".to_string(),
                    actual: format!("{:?}", executable.tree.get(ty)),
                },
            ));
        }

        Ok(())
    }

    /// Convert a MIR constant to a runtime value.
    fn constant_to_value(&mut self, constant: &mir::Constant) -> Value {
        Value::from(constant)
    }

    /// Create a zero value for a given type.
    fn zero_value(
        &mut self,
        executable: &Executable,
        mut memory: MemoryContext<'_>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        // resolve the type node
        let ty_node = executable.tree.get(ty).clone();

        // build a zero value based on type
        match ty_node {
            mir::Type::Void => Ok(Value::VOID),
            mir::Type::Int { width, is_signed } => {
                // select signed or unsigned zero
                if is_signed {
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
            mir::Type::TypeDescriptor | mir::Type::TypeId => Err(self.make_error(
                executable,
                Error::UnsupportedZeroValue {
                    ty: format!("{ty_node:?}"),
                },
            )),
            mir::Type::Reference {
                kind,
                address_space,
                mutability,
                is_nullable,
                ..
            } => {
                if !is_nullable {
                    return Err(self.make_error(
                        executable,
                        Error::UnsupportedZeroValue {
                            ty: format!("{ty_node:?}"),
                        },
                    ));
                }

                let meta = ReferenceMeta::new(kind, address_space, mutability, is_nullable);
                match kind {
                    mir::ReferenceKind::Managed => Ok(Value::managed_reference_with_meta(
                        ManagedReference::NULL,
                        meta,
                    )),
                    mir::ReferenceKind::Owned
                    | mir::ReferenceKind::Borrowed
                    | mir::ReferenceKind::Raw
                        if matches!(
                            meta.address_space(),
                            destack_heap::ReferenceAddressSpace::Shared
                        ) =>
                    {
                        Ok(Value::shared_pointer_with_meta(SharedPointer::NULL, meta))
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
                    .map(|e| self.zero_value(executable, memory.reborrow(), e))
                    .collect::<RuntimeResult<_>>()?;

                // allocate managed aggregate for tuple
                let handle = memory
                    .heap()
                    .allocate_packed_values(values)
                    .map_err(|error| self.make_error(executable, Error::from(error)))?;

                Ok(Value::aggregate(handle))
            }
            mir::Type::Array {
                element,
                length,
                copyability: _,
            } => {
                // build an array of repeated element zeros
                let elem_zero = self.zero_value(executable, memory.reborrow(), element)?;
                let values: Vec<Value> = (0..length).map(|_| elem_zero).collect();

                // allocate managed aggregate for array
                let handle = memory
                    .heap()
                    .allocate_packed_values(values)
                    .map_err(|error| self.make_error(executable, Error::from(error)))?;

                Ok(Value::aggregate(handle))
            }
            _ => Err(self.make_error(
                executable,
                Error::UnsupportedZeroValue {
                    ty: format!("{ty_node:?}"),
                },
            )),
        }
    }

    /// Get call stack info for error reporting.
    fn get_call_stack_info(&self, executable: &Executable) -> Vec<FrameInfo> {
        self.call_stack
            .iter()
            .map(|f| {
                let func = executable.tree.get(f.function);
                let name = executable.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Run garbage collection on the managed heap.
    pub(crate) fn collect_garbage(
        &mut self,
        executable: &Executable,
        string_interner: &mut StringInterner,
        globals: &GlobalStorage,
        memory: MemoryContext<'_>,
    ) -> GcStats {
        self.collect_garbage_with_continuations(executable, string_interner, globals, memory, &[])
    }

    /// Run garbage collection including suspended continuations.
    pub(crate) fn collect_garbage_with_continuations(
        &mut self,
        executable: &Executable,
        string_interner: &mut StringInterner,
        globals: &GlobalStorage,
        mut memory: MemoryContext<'_>,
        continuations: &[Continuation],
    ) -> GcStats {
        // collect roots from active frames
        let mut roots = Vec::new();
        for frame in &self.call_stack {
            frame.collect_roots(executable, &self.value_stack, &self.local_stack, &mut roots);
        }

        // collect roots from continuations
        for continuation in continuations {
            continuation.collect_roots(executable, &mut roots);
        }

        // collect roots from globals
        for value in globals.values() {
            if let Some(handle) = value.as_managed_reference() {
                roots.push(handle);
            }
        }

        // collect roots from interned string literals
        string_interner.collect_roots(&mut roots);

        // run collection
        let stats = memory.heap().collect_managed_handles(roots);

        // sweep raw payload buffers for freed strings
        string_interner.sweep_buffers(memory.heap());

        stats
    }
}
