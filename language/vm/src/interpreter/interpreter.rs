#[cfg(feature = "stats")]
use std::time::Duration;

use destack_mir as mir;

use crate::diagnostic::{Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::interpreter::Continuation;
use crate::isolate::{ExternalCallContext, GlobalStorage, RootVisitor};
use crate::module::{FunctionTable, Module};
use crate::snapshot::InterpreterImage;
use crate::telemetry::Statistics;
use crate::{ReferenceAddressSpace, ReferenceMeta, SharedHeap, Value};
use destack_heap::{
    Heap, HeapReference, RawPointer, SharedHeapReference, SharedRawLimits, SharedRawPointer,
};

use super::Frame;
#[cfg(feature = "stats")]
use crate::telemetry::InstructionProfile;

/// Interpreter execution engine.
#[derive(Debug)]
pub struct Interpreter {
    /// Explicit frame stack used for execution and root walking.
    pub(crate) stack: Vec<Frame>,
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
            stack: Vec::new(),
            statistics: Statistics::new(),
            #[cfg(feature = "stats")]
            instruction_profile: None,
        }
    }

    /// Get the current frame stack for this engine.
    pub(crate) fn stack(&self) -> &[Frame] {
        &self.stack
    }

    /// Capture one immutable interpreter image.
    pub(crate) fn image(&self) -> InterpreterImage {
        let stack = self.stack.iter().map(Frame::image).collect();

        InterpreterImage {
            stack,
            statistics: self.statistics.clone(),
        }
    }

    /// Fork this interpreter for one child isolate.
    pub(crate) fn fork(&self) -> Self {
        // clone the live frame stack for the child isolate
        let stack = self.stack.iter().map(Frame::clone_for_fork).collect();
        Self {
            stack,
            statistics: self.statistics.clone(),
            #[cfg(feature = "stats")]
            instruction_profile: None,
        }
    }

    /// Create one interpreter from an immutable image.
    pub(crate) fn from_image(
        functions: &FunctionTable,
        image: &InterpreterImage,
    ) -> RuntimeResult<Self> {
        let mut interpreter = Self::new();

        // restore the mutable execution state
        let stack = image
            .stack
            .iter()
            .map(|frame| Frame::from_image(frame, functions))
            .collect::<RuntimeResult<Vec<_>>>()?;

        interpreter.stack = stack;
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

    /// Create an error with current call stack.
    #[cold]
    pub(crate) fn make_error(&self, module: &Module, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info(module))
    }

    /// Initialize global variables from the MIR tree.
    pub(crate) fn initialize_globals(
        &mut self,
        module: &Module,
        globals: &mut GlobalStorage,
        heap: &mut Heap,
        shared: &SharedHeap,
    ) -> RuntimeResult<()> {
        // seed empty global storage
        let mut initialized_globals = GlobalStorage::new();

        // snapshot globals to avoid borrowing module during initialization
        let global_entries: Vec<_> = module
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

            // materialize initializer when present
            let value = match initializer.as_ref() {
                Some(init) => {
                    self.materialize_global_initializer(module, heap, shared, init, ty)?
                }
                None => Value::VOID,
            };
            initialized_globals.set(id, value);
        }

        // store initialized globals
        *globals = initialized_globals;

        Ok(())
    }

    /// Materialize one global initializer into its declared heap value.
    fn materialize_global_initializer(
        &mut self,
        module: &Module,
        heap: &mut Heap,
        shared: &SharedHeap,
        init: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        // select initializer strategy
        match init {
            mir::GlobalInitializer::Zero => self.zero_value(module, heap, shared, ty),
            mir::GlobalInitializer::Scalar(constant) => Ok(self.scalar_constant_value(constant)),
            mir::GlobalInitializer::Bytes(bytes) => {
                // materialize bytes as u8 values
                let values: Vec<Value> = bytes.iter().map(|&b| Value::uint(b as u64, 8)).collect();

                // allocate heap value for the declared global type
                self.allocate_initializer_heap_value(module, heap, shared, ty, values)
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                // resolve the declared aggregate value types
                let value_types = self.aggregate_member_types(module, ty)?;
                if elements.len() != value_types.len() {
                    return Err(self.make_error(
                        module,
                        Error::TypeMismatch {
                            expected: format!(
                                "initializer with {} aggregate values",
                                value_types.len()
                            ),
                            actual: format!("initializer with {} aggregate values", elements.len()),
                        },
                    ));
                }

                // materialize each element using the declared value type
                let values = elements
                    .iter()
                    .zip(value_types.into_iter())
                    .map(|(element, value_type)| {
                        self.materialize_global_initializer(
                            module, heap, shared, element, value_type,
                        )
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?;

                // allocate heap value for the declared global type
                self.allocate_initializer_heap_value(module, heap, shared, ty, values)
            }
        }
    }

    /// Resolve the declared aggregate member types for one layout-backed aggregate.
    fn aggregate_member_types(
        &self,
        module: &Module,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Vec<mir::LocalNodeId<mir::Type>>> {
        let layout = module.layout(ty).ok_or_else(|| {
            self.make_error(
                module,
                Error::TypeMismatch {
                    expected: "layout-backed aggregate type".to_string(),
                    actual: format!("{ty:?}"),
                },
            )
        })?;
        if let Some(field_count) = layout.field_count() {
            let mut value_types = Vec::with_capacity(field_count);

            // collect field types in source order
            for index in 0..field_count {
                let field = layout.field(index as u32).ok_or_else(|| {
                    self.make_error(
                        module,
                        Error::TypeMismatch {
                            expected: "field layout".to_string(),
                            actual: format!("{ty:?}"),
                        },
                    )
                })?;
                value_types.push(field.ty);
            }

            return Ok(value_types);
        }

        let Some(element) = layout.element() else {
            return Err(self.make_error(
                module,
                Error::TypeMismatch {
                    expected: "aggregate layout".to_string(),
                    actual: format!("{ty:?}"),
                },
            ));
        };
        let element_count = layout.element_count().ok_or_else(|| {
            self.make_error(
                module,
                Error::TypeMismatch {
                    expected: "indexed layout".to_string(),
                    actual: format!("{ty:?}"),
                },
            )
        })?;

        Ok(vec![element.ty; element_count])
    }

    /// Allocate one initializer aggregate through the heap allocation path.
    fn allocate_initializer_heap_value(
        &mut self,
        module: &Module,
        heap: &mut Heap,
        shared: &SharedHeap,
        ty: mir::LocalNodeId<mir::Type>,
        values: Vec<Value>,
    ) -> RuntimeResult<Value> {
        let mut context =
            ExternalCallContext::new(module, heap, shared, SharedRawLimits::default());
        context
            .materialize_heap_value(ty, values)
            .map_err(|error| self.make_error(module, error))
    }

    /// Build one scalar runtime value from one MIR constant.
    fn scalar_constant_value(&mut self, constant: &mir::Constant) -> Value {
        Value::from(constant)
    }

    /// Create a zero value for a given type.
    fn zero_value(
        &mut self,
        module: &Module,
        heap: &mut Heap,
        shared: &SharedHeap,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        // materialize one zeroed aggregate through the heap path
        if module.layout(ty).is_some_and(|layout| !layout.is_scalar()) {
            let value_types = self.aggregate_member_types(module, ty)?;
            let values = value_types
                .into_iter()
                .map(|value_type| self.zero_value(module, heap, shared, value_type))
                .collect::<RuntimeResult<Vec<_>>>()?;

            return self.allocate_initializer_heap_value(module, heap, shared, ty, values);
        }

        // resolve the type node
        let ty_node = module.tree.get(ty).clone();

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
                module,
                Error::UnsupportedZeroValue {
                    ty: format!("{ty_node:?}"),
                },
            )),
            mir::Type::Reference {
                kind,
                ref address_space,
                mutability,
                is_nullable,
                ..
            } => {
                if !is_nullable {
                    return Err(self.make_error(
                        module,
                        Error::UnsupportedZeroValue {
                            ty: format!("{ty_node:?}"),
                        },
                    ));
                }

                let meta = ReferenceMeta::new(kind, address_space.clone(), mutability, is_nullable);
                match kind {
                    mir::ReferenceKind::Managed | mir::ReferenceKind::Owned
                        if matches!(meta.address_space(), ReferenceAddressSpace::Shared) =>
                    {
                        Ok(Value::shared_heap_reference_with_meta(
                            SharedHeapReference::NULL,
                            meta,
                        ))
                    }
                    mir::ReferenceKind::Managed | mir::ReferenceKind::Owned => {
                        Ok(Value::heap_reference_with_meta(HeapReference::NULL, meta))
                    }
                    mir::ReferenceKind::Borrowed | mir::ReferenceKind::Raw
                        if matches!(meta.address_space(), ReferenceAddressSpace::Shared) =>
                    {
                        Ok(Value::shared_raw_pointer_with_meta(
                            SharedRawPointer::NULL,
                            meta,
                        ))
                    }
                    mir::ReferenceKind::Borrowed | mir::ReferenceKind::Raw => {
                        Ok(Value::raw_pointer_with_meta(RawPointer::NULL, meta))
                    }
                }
            }
            _ => Err(self.make_error(
                module,
                Error::UnsupportedZeroValue {
                    ty: format!("{ty_node:?}"),
                },
            )),
        }
    }

    /// Get call stack info for error reporting.
    fn get_call_stack_info(&self, module: &Module) -> Vec<FrameInfo> {
        self.stack
            .iter()
            .map(|f| {
                let func = module.tree.get(f.function);
                let name = module.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Visit one complete root set from active state and suspended continuations.
    pub(crate) fn visit_roots(
        &mut self,
        module: &Module,
        globals: &GlobalStorage,
        continuations: &[Continuation],
        roots: &mut impl RootVisitor,
    ) -> RuntimeResult<()> {
        // active frames
        for frame in &self.stack {
            frame
                .visit_roots(module, roots)
                .map_err(|error| self.make_error(module, error))?;
        }

        // suspended continuations
        for continuation in continuations {
            continuation
                .visit_roots(module, roots)
                .map_err(|error| self.make_error(module, error))?;
        }

        // globals
        for value in globals.values() {
            if let Some(reference) = value.as_heap_reference() {
                roots.push_heap(reference);
            }

            if let Some(reference) = value.as_shared_heap_reference() {
                roots.push_shared(reference);
            }
        }

        Ok(())
    }
}
