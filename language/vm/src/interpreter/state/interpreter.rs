use std::collections::HashMap;
#[cfg(feature = "stats")]
use std::time::Duration;

use destack_mir as mir;

use crate::diagnostic::{Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::executable::{Executable, FunctionTable};
use crate::interpreter::Continuation;
use crate::isolate::{
    ExternalCallContext, ExternalFn, GlobalStorage, SchemaRegistry, StringInterner,
};
use crate::snapshot::InterpreterImage;
use crate::telemetry::Statistics;
use destack_heap::{
    GcStats, ManagedReference, MemoryContext, RawPointer, ReferenceMeta, SharedPointer, Value,
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
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Result<ExternalFn, RuntimeError> {
        let func = executable.tree.get(function_id);
        let name = executable.strings.get(func.name).to_string();
        externals
            .get(&name)
            .cloned()
            .ok_or_else(|| self.make_error(executable, Error::ExternalFunctionNotFound { name }))
    }

    /// Create an error with current call stack.
    #[cold]
    pub(crate) fn make_error(&self, executable: &Executable, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info(executable))
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
                let ty = (global.ty).ty().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "global type".to_string(),
                })?;

                Ok((id, ty, global.is_import(), global.initializer.clone()))
            })
            .collect::<crate::Result<Vec<_>>>()?;

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
            mir::GlobalInitializer::Zero => {
                self.zero_value(executable, string_interner, memory.reborrow(), ty)
            }
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

                // materialize typed storage for the declared global type
                self.materialize_storage_value(
                    executable,
                    string_interner,
                    memory.reborrow(),
                    ty,
                    values,
                )
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                // resolve the declared component types
                let component_types = self.storage_component_types(executable, ty)?;
                if elements.len() != component_types.len() {
                    return Err(self.make_error(
                        executable,
                        Error::TypeMismatch {
                            expected: format!(
                                "initializer with {} composite components",
                                component_types.len()
                            ),
                            actual: format!(
                                "initializer with {} composite components",
                                elements.len()
                            ),
                        },
                    ));
                }

                // convert each element recursively using the declared component type
                let values = elements
                    .iter()
                    .zip(component_types.into_iter())
                    .map(|(element, component_type)| {
                        self.convert_initializer(
                            executable,
                            string_interner,
                            memory.reborrow(),
                            element,
                            component_type,
                        )
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?;

                // materialize typed storage for the declared global type
                self.materialize_storage_value(
                    executable,
                    string_interner,
                    memory.reborrow(),
                    ty,
                    values,
                )
            }
        }
    }

    /// Resolve the declared storage component types for one layout-backed type.
    fn storage_component_types(
        &self,
        executable: &Executable,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Vec<mir::LocalNodeId<mir::Type>>> {
        let layout = executable.layout(ty).ok_or_else(|| {
            self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "layout-backed composite type".to_string(),
                    actual: format!("{ty:?}"),
                },
            )
        })?;
        let component_count = layout.component_count().ok_or_else(|| {
            self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "composite layout".to_string(),
                    actual: format!("{ty:?}"),
                },
            )
        })?;
        let mut component_types = Vec::with_capacity(component_count);

        // collect component types in semantic order
        for index in 0..component_count {
            let component = layout.component(index as u32).ok_or_else(|| {
                self.make_error(
                    executable,
                    Error::TypeMismatch {
                        expected: "storage component".to_string(),
                        actual: format!("{ty:?}"),
                    },
                )
            })?;
            component_types.push(component.ty);
        }

        Ok(component_types)
    }

    /// Materialize one layout-backed value through the shared typed allocator.
    fn materialize_storage_value(
        &mut self,
        executable: &Executable,
        string_interner: &mut StringInterner,
        mut memory: MemoryContext<'_>,
        ty: mir::LocalNodeId<mir::Type>,
        values: Vec<Value>,
    ) -> RuntimeResult<Value> {
        let schema = SchemaRegistry::new().map_err(|error| self.make_error(executable, error))?;
        let mut context =
            ExternalCallContext::new(executable, &schema, string_interner, memory.reborrow());
        context
            .materialize_storage_value_for_type(ty, values)
            .map_err(|error| self.make_error(executable, error))
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
                    expected: "ref<String, managed>".to_string(),
                    actual: format!("{:?}", executable.tree.get(ty)),
                },
            ));
        };

        // ensure the string is a managed reference
        if *kind != mir::ReferenceKind::Managed {
            return Err(self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "ref<String, managed>".to_string(),
                    actual: format!("{:?}", executable.tree.get(ty)),
                },
            ));
        }

        // validate the reference points at the canonical well known String layout
        let Some(string_type) = executable.tree.string_type() else {
            return Err(self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "canonical well known String".to_string(),
                    actual: "missing".to_string(),
                },
            ));
        };
        let expected_pointee = match executable.tree.get(string_type) {
            mir::Type::Reference { pointee, .. } => {
                (*pointee).ty().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "string pointee type".to_string(),
                })?
            }
            _ => string_type,
        };

        let pointee = (*pointee).ty().ok_or_else(|| Error::ConcreteMirRequired {
            context: "string initializer pointee".to_string(),
        })?;

        if pointee != expected_pointee {
            return Err(self.make_error(
                executable,
                Error::TypeMismatch {
                    expected: "ref<String, managed>".to_string(),
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
        string_interner: &mut StringInterner,
        mut memory: MemoryContext<'_>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        // materialize one zeroed composite through the typed storage path
        if executable
            .layout(ty)
            .is_some_and(|layout| !layout.is_scalar())
        {
            let component_types = self.storage_component_types(executable, ty)?;
            let values = component_types
                .into_iter()
                .map(|component_type| {
                    self.zero_value(
                        executable,
                        string_interner,
                        memory.reborrow(),
                        component_type,
                    )
                })
                .collect::<RuntimeResult<Vec<_>>>()?;

            return self.materialize_storage_value(
                executable,
                string_interner,
                memory.reborrow(),
                ty,
                values,
            );
        }

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
    ) -> RuntimeResult<GcStats> {
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
    ) -> RuntimeResult<GcStats> {
        // collect roots from active frames
        let mut roots = Vec::new();
        for frame in &self.call_stack {
            frame
                .collect_roots(executable, &self.value_stack, &self.local_stack, &mut roots)
                .map_err(|error| self.make_error(executable, error))?;
        }

        // collect roots from continuations
        for continuation in continuations {
            continuation
                .collect_roots(executable, &mut roots)
                .map_err(|error| self.make_error(executable, error))?;
        }

        // collect roots from globals
        for value in globals.values() {
            if let Some(handle) = value.as_managed_reference() {
                roots.push(handle);
            }
        }

        // collect roots from interned string literals
        string_interner.collect_roots(&mut roots);

        // perform staged collection
        let stats = memory.heap().collect_managed_handles_staged(roots);

        // sweep raw payload buffers for freed strings
        string_interner.sweep_buffers(memory.heap());

        Ok(stats)
    }
}
