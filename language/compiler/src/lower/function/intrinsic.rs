use destack_dir as dir;
use destack_mir as mir;
use destack_mir::{IntrinsicInstruction, IntrinsicTerminator};

use crate::lower::{ContextIntrinsic, FunctionLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// The target constant one intrinsic name folds to.
enum LayoutIntrinsic {
    /// The size of the subject type.
    SizeOf,
    /// The alignment of the subject type.
    AlignOf,
    /// The padded element step of the subject type.
    StrideOf,
    /// A well-aligned dangling pointer to the subject type.
    Dangling,
    /// A pointer moved by an element count.
    Offset,
    /// The element distance between two pointers.
    OffsetFrom,
}

impl LayoutIntrinsic {
    /// Return the layout intrinsic one name denotes.
    fn from_name(name: &str) -> Option<Self> {
        let denoted = match name {
            "reflect.sizeOf" => Self::SizeOf,
            "reflect.alignOf" => Self::AlignOf,
            "reflect.strideOf" => Self::StrideOf,
            "memory.ptr.dangling" => Self::Dangling,
            "memory.ptr.offset" | "memory.ptr.wrappingOffset" => Self::Offset,
            "memory.ptr.offsetFrom" => Self::OffsetFrom,
            _ => return None,
        };

        Some(denoted)
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one intrinsic call through its name's vocabulary.
    pub(in crate::lower) fn lower_intrinsic_call(
        &mut self,
        name: Option<String>,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(name) = name else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an unnamed intrinsic callable".to_string(),
            }
            .into());
        };

        // route the name through the machine operations, then the instructions
        if let Ok(operation) = name.parse::<mir::Intrinsic>() {
            return self.lower_operation_intrinsic(operation, resolution);
        }
        if let Some(instruction) = IntrinsicInstruction::from_name(&name) {
            return self.lower_instruction_intrinsic(instruction, resolution);
        }
        if let Some(terminator) = IntrinsicTerminator::from_name(&name) {
            return self.lower_terminator_intrinsic(terminator, resolution);
        }
        if let Some(layout) = LayoutIntrinsic::from_name(&name) {
            return self.lower_layout_intrinsic(layout, resolution);
        }
        if let Some(context) = ContextIntrinsic::from_name(&name) {
            return self.lower_context_intrinsic(context, resolution);
        }

        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: format!("the '{name}' intrinsic"),
        }
        .into())
    }

    /// Lower one operation intrinsic.
    fn lower_operation_intrinsic(
        &mut self,
        operation: mir::Intrinsic,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let values = self.lower_call_arguments(&resolution.arguments, &[], None)?;

        if !operation.has_result() {
            self.builder.intrinsic_void(operation, values);

            return Ok(None);
        }

        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.intrinsic(operation, result, values)))
    }

    /// Lower one instruction intrinsic through its family emitter.
    fn lower_instruction_intrinsic(
        &mut self,
        instruction: IntrinsicInstruction,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        match instruction {
            IntrinsicInstruction::AtomicFence => self.lower_atomic_fence(resolution),
            IntrinsicInstruction::AtomicLoad => self.lower_atomic_load(resolution),
            IntrinsicInstruction::AtomicCompareExchange { weak } => {
                self.lower_atomic_compare_exchange(weak, resolution)
            }
            IntrinsicInstruction::AtomicRmw(operator) => {
                self.lower_atomic_rmw(operator, resolution)
            }
            IntrinsicInstruction::AtomicStore => self.lower_atomic_store(resolution),
            IntrinsicInstruction::Breakpoint => {
                self.builder.breakpoint();

                Ok(None)
            }
            IntrinsicInstruction::Cast(operator) => self.lower_cast_intrinsic(operator, resolution),
            IntrinsicInstruction::PointerLoad => self.lower_pointer_load(resolution),
            IntrinsicInstruction::PointerStore => self.lower_pointer_store(resolution),
            IntrinsicInstruction::PointerReplace => self.lower_pointer_replace(resolution),
            IntrinsicInstruction::PointerSwap => self.lower_pointer_swap(resolution),
            IntrinsicInstruction::PointerDropInPlace => {
                self.lower_pointer_drop_in_place(resolution)
            }
            IntrinsicInstruction::Drop => {
                let value = self.argument_value(resolution, 0)?;
                self.builder.drop_value(value);

                Ok(None)
            }
            IntrinsicInstruction::InitUninit => {
                self.lower_storage_constant(mir::Constant::Uninit, resolution)
            }
            IntrinsicInstruction::InitZeroed => {
                self.lower_storage_constant(mir::Constant::Zeroed, resolution)
            }
            IntrinsicInstruction::InitWrite => self.lower_init_write(resolution),
            IntrinsicInstruction::SliceFromRaw => self.lower_slice_from_raw(resolution),
            IntrinsicInstruction::SliceGet => self.lower_slice_get(resolution),
            IntrinsicInstruction::SliceLength => self.lower_slice_length(resolution),
            IntrinsicInstruction::SliceSet => self.lower_slice_set(resolution),
            IntrinsicInstruction::SliceView => self.lower_slice_view(resolution),
        }
    }

    /// Lower one terminator intrinsic and open a dead block for trailing code.
    fn lower_terminator_intrinsic(
        &mut self,
        terminator: IntrinsicTerminator,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        match terminator {
            IntrinsicTerminator::Abort => {
                let values = self.lower_call_arguments(&resolution.arguments, &[], None)?;

                self.builder.abort(values.into_iter().next());
            }
            IntrinsicTerminator::Unreachable => self.builder.unreachable(),
            IntrinsicTerminator::Panic => {
                let values = self.lower_call_arguments(&resolution.arguments, &[], None)?;

                self.builder.panic(values.into_iter().next());
            }
        }

        // continue building into an unreachable block after the terminator
        let dead = self.builder.block();
        self.builder.switch_to_block(dead);

        Ok(None)
    }

    /// Lower one atomic fence from its const ordering configuration.
    ///
    /// Region and memory-scope refinements subsume into a whole-storage fence.
    pub(in crate::lower) fn lower_atomic_fence(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let access = self.atomic_access(resolution, 0)?;
        self.builder.atomic_fence(mir::FenceAccess {
            ordering: access.ordering,
            scope: access.scope,
            storage: mir::StorageSet::ANY,
        });

        Ok(None)
    }

    /// Lower one atomic load from its pointer and const configuration.
    pub(in crate::lower) fn lower_atomic_load(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let access = self.atomic_access(resolution, 1)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.atomic_load(pointer, access, result)))
    }

    /// Lower one atomic store from its pointer, value, and configuration.
    pub(in crate::lower) fn lower_atomic_store(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let value = self.argument_value(resolution, 1)?;
        let access = self.atomic_access(resolution, 2)?;
        self.builder.atomic_store(pointer, value, access);

        Ok(None)
    }

    /// Lower one atomic read-modify-write returning the old value.
    pub(in crate::lower) fn lower_atomic_rmw(
        &mut self,
        operator: mir::AtomicRmwOperator,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let value = self.argument_value(resolution, 1)?;
        let access = self.atomic_access(resolution, 2)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(
            self.builder
                .atomic_rmw(operator, pointer, value, access, result),
        ))
    }

    /// Lower one atomic compare-exchange returning the old value and success.
    ///
    /// The failed comparison loads at the success ordering stripped of its
    /// release half.
    pub(in crate::lower) fn lower_atomic_compare_exchange(
        &mut self,
        weak: bool,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let expected = self.argument_value(resolution, 1)?;
        let desired = self.argument_value(resolution, 2)?;
        let access = self.atomic_access(resolution, 3)?;
        let failure_ordering = match access.ordering {
            mir::MemoryOrdering::Release => mir::MemoryOrdering::Relaxed,
            mir::MemoryOrdering::AcquireRelease => mir::MemoryOrdering::Acquire,
            ordering => ordering,
        };
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.atomic_compare_exchange(
            pointer,
            expected,
            desired,
            weak,
            mir::CompareExchangeAccess::new(access, failure_ordering),
            result,
        )))
    }

    /// Lower one scalar cast of its sole operand.
    pub(in crate::lower) fn lower_cast_intrinsic(
        &mut self,
        operator: mir::CastOperator,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let operand = self.argument_value(resolution, 0)?;
        let target = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.cast(operator, operand, target)))
    }

    /// Lower one slice view over a raw address and length.
    pub(in crate::lower) fn lower_slice_from_raw(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let data = self.argument_value(resolution, 0)?;
        let length = self.argument_value(resolution, 1)?;
        let start = self.builder.iconst(0, 64, false);
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.slice_view(data, start, length, result)))
    }

    /// Lower one slice element read.
    pub(in crate::lower) fn lower_slice_get(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;
        let index = self.argument_value(resolution, 1)?;
        let element = self.lower_type(resolution.return_type)?;
        let pointer = self.emit_element_address(slice, index, element)?;

        Ok(Some(self.builder.load(pointer, element)))
    }

    /// Lower one slice element write.
    pub(in crate::lower) fn lower_slice_set(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;
        let index = self.argument_value(resolution, 1)?;
        let value = self.argument_value(resolution, 2)?;
        let element = self.value_carrier(value)?;
        let pointer = self.emit_element_address(slice, index, element)?;
        self.builder.store(pointer, value);

        Ok(None)
    }

    /// Address one slice element behind an exclusive borrow carrier.
    fn emit_element_address(
        &mut self,
        slice: mir::Value,
        index: mir::Value,
        element: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        let pointer = self.insert_reference(
            mir::ReferenceKind::Borrowed,
            mir::Access::Exclusive,
            element,
        );

        Ok(self.builder.element_addr(slice, index, pointer))
    }

    /// Lower one storage constant typed by its declared return.
    fn lower_storage_constant(
        &mut self,
        constant: mir::Constant,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.constant(constant, result)))
    }

    /// Lower one initializing store returning the initialized borrow.
    fn lower_init_write(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let value = self.argument_value(resolution, 1)?;
        self.builder.store(pointer, value);
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.intrinsic(
            mir::Intrinsic::Transmute,
            result,
            vec![pointer],
        )))
    }

    /// Lower one load through a raw pointer.
    pub(in crate::lower) fn lower_pointer_load(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.load(pointer, result)))
    }

    /// Lower one store through a raw pointer.
    pub(in crate::lower) fn lower_pointer_store(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let value = self.argument_value(resolution, 1)?;
        self.builder.store(pointer, value);

        Ok(None)
    }

    /// Lower one pointed-to value exchange returning the old value.
    pub(in crate::lower) fn lower_pointer_replace(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let value = self.argument_value(resolution, 1)?;
        let result = self.lower_type(resolution.return_type)?;
        let old = self.builder.load(pointer, result);
        self.builder.store(pointer, value);

        Ok(Some(old))
    }

    /// Lower one swap of two pointed-to values.
    pub(in crate::lower) fn lower_pointer_swap(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let first = self.argument_value(resolution, 0)?;
        let second = self.argument_value(resolution, 1)?;
        let pointee = self.pointee_type(first)?;
        let first_value = self.builder.load(first, pointee);
        let second_value = self.builder.load(second, pointee);
        self.builder.store(first, second_value);
        self.builder.store(second, first_value);

        Ok(None)
    }

    /// Lower one drop of the pointed-to value.
    pub(in crate::lower) fn lower_pointer_drop_in_place(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let pointee = self.pointee_type(pointer)?;
        let value = self.builder.load(pointer, pointee);
        self.builder.drop_value(value);

        Ok(None)
    }

    /// Return the pointee type behind one lowered pointer value.
    fn pointee_type(&mut self, pointer: mir::Value) -> CompilerResult<mir::TypeId> {
        let pointer_type = self.value_carrier(pointer)?;
        let pointee = match self.builder.tree().get(pointer_type) {
            mir::Type::Pointer { pointee, .. } | mir::Type::Reference { pointee, .. } => pointee,
            _ => {
                return Err(CompilerError::Internal {
                    message: "a pointer operation on a value without a pointee".to_string(),
                });
            }
        };

        Ok(*pointee)
    }

    /// Lower one slice length read.
    pub(in crate::lower) fn lower_slice_length(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;

        Ok(Some(self.builder.slice_length(slice)))
    }

    /// Lower one slice view over a contiguous range.
    pub(in crate::lower) fn lower_slice_view(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;
        let start = self.argument_value(resolution, 1)?;
        let length = self.argument_value(resolution, 2)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.slice_view(slice, start, length, result)))
    }

    /// Read one atomic configuration starting at one argument position.
    fn atomic_access(
        &mut self,
        resolution: &dir::Call,
        start: usize,
    ) -> CompilerResult<mir::AtomicAccess> {
        let ordering = match self.const_case(resolution, start, "an atomic ordering")? {
            0 => mir::MemoryOrdering::Relaxed,
            1 => mir::MemoryOrdering::Acquire,
            2 => mir::MemoryOrdering::Release,
            3 => mir::MemoryOrdering::AcquireRelease,
            _ => mir::MemoryOrdering::SequentiallyConsistent,
        };
        let scope = match self.const_case(resolution, start + 1, "an atomic scope")? {
            0 => mir::ExecutionScope::Invocation,
            1 => mir::ExecutionScope::Subgroup,
            2 => mir::ExecutionScope::Workgroup,
            3 => mir::ExecutionScope::Device,
            7 => mir::ExecutionScope::System,
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("the atomic scope in declaration order {other}"),
                }
                .into());
            }
        };

        // reject the three trailing flags
        for (offset, flag) in [
            (4, "a volatile atomic"),
            (5, "an availability-publishing atomic"),
            (6, "a visibility-acquiring atomic"),
        ] {
            if self.const_boolean(resolution, start + offset, flag)? {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: flag.to_string(),
                }
                .into());
            }
        }

        Ok(mir::AtomicAccess::new(ordering, scope))
    }

    /// Lower one layout intrinsic to its folded target constants.
    fn lower_layout_intrinsic(
        &mut self,
        layout: LayoutIntrinsic,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer_bits = self.builder.pointer_bits();
        let value = match layout {
            LayoutIntrinsic::SizeOf => {
                let layout = self.subject_layout(resolution)?;

                self.builder
                    .iconst(layout.size as i128, pointer_bits, false)
            }
            LayoutIntrinsic::AlignOf => {
                let layout = self.subject_layout(resolution)?;

                self.builder
                    .iconst(layout.alignment as i128, pointer_bits, false)
            }
            LayoutIntrinsic::StrideOf => {
                let stride = self.subject_stride(resolution)?;

                self.builder.iconst(stride as i128, pointer_bits, false)
            }
            LayoutIntrinsic::Dangling => {
                let layout = self.subject_layout(resolution)?;
                let pointer = self.lower_type(resolution.return_type)?;
                let address =
                    self.builder
                        .iconst(layout.alignment.max(1) as i128, pointer_bits, false);

                self.builder
                    .intrinsic(mir::Intrinsic::Transmute, pointer, vec![address])
            }
            LayoutIntrinsic::Offset => {
                let stride = self.subject_stride(resolution)?;
                let pointer = self.argument_value(resolution, 0)?;
                let count = self.argument_value(resolution, 1)?;
                let step = self.builder.iconst(stride as i128, pointer_bits, true);
                let domain = self.value_carrier(step)?;
                let address =
                    self.builder
                        .intrinsic(mir::Intrinsic::Transmute, domain, vec![pointer]);
                let bytes = self
                    .builder
                    .binary(mir::BinaryOperator::Multiply, count, step);
                let moved = self
                    .builder
                    .binary(mir::BinaryOperator::Add, address, bytes);
                let result = self.lower_type(resolution.return_type)?;

                self.builder
                    .intrinsic(mir::Intrinsic::Transmute, result, vec![moved])
            }
            LayoutIntrinsic::OffsetFrom => {
                let stride = self.subject_stride(resolution)?;
                let pointer = self.argument_value(resolution, 0)?;
                let origin = self.argument_value(resolution, 1)?;
                let step = self.builder.iconst(stride as i128, pointer_bits, true);
                let domain = self.value_carrier(step)?;
                let bytes = self.builder.intrinsic(
                    mir::Intrinsic::PointerByteOffsetFrom,
                    domain,
                    vec![pointer, origin],
                );

                self.builder
                    .binary(mir::BinaryOperator::Divide, bytes, step)
            }
        };

        Ok(Some(value))
    }

    /// Return the folded layout of one call's subject type argument.
    fn subject_layout(&mut self, resolution: &dir::Call) -> CompilerResult<mir::Layout> {
        let dir::Call {
            target: dir::CallableTarget::Symbol { function, .. },
            ..
        } = resolution
        else {
            return Err(CompilerError::Internal {
                message: "a layout intrinsic without a candidate".to_string(),
            });
        };
        let bindings = self
            .lowerer
            .instance_bindings(&function.selection.arguments, self.instance)?;
        let Some(subject) = bindings.first().map(|binding| binding.argument) else {
            return Err(CompilerError::Internal {
                message: "a layout intrinsic without a subject type".to_string(),
            });
        };
        let subject = self.lower_type(subject)?;

        // lay the subject out at the target to answer the query
        let pointer_bytes = (self.builder.pointer_bits() / 8) as u8;
        let target = mir::TargetLayout::for_pointer_bytes(pointer_bytes);
        let mut layouts = mir::LayoutTable::default();
        let mut builder = mir::LayoutBuilder::new(self.builder.tree_mut(), &mut layouts, target);
        let id = builder
            .layout_type(subject)
            .map_err(|error| CompilerError::from((self.lowerer.module, error)))?;

        Ok(layouts.layout(id).clone())
    }

    /// Return the padded element step of one call's subject type argument.
    fn subject_stride(&mut self, resolution: &dir::Call) -> CompilerResult<u32> {
        let layout = self.subject_layout(resolution)?;

        Ok(layout.size.next_multiple_of(layout.alignment.max(1)))
    }

    /// Lower one provided argument's value expression.
    pub(in crate::lower) fn argument_value(
        &mut self,
        resolution: &dir::Call,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        let source = self.argument_expression(resolution, index)?;

        self.lower_expression(source)
    }

    /// Read one const enum argument as its declared case ordinal.
    fn const_case(
        &mut self,
        resolution: &dir::Call,
        index: usize,
        construct: &str,
    ) -> CompilerResult<u32> {
        let source = self.argument_expression(resolution, index)?;
        let dir::Type::Variant(member) = self.node_type(source)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("{construct} without a const-evaluated value"),
            }
            .into());
        };
        let dir::Type::Application(owner) = self.lowerer.ty(member.owner)? else {
            return Err(CompilerError::Internal {
                message: "an enum member without its owner instance".to_string(),
            });
        };

        self.lowerer.variant_position(owner.symbol, member.variant)
    }

    /// Read one const boolean argument.
    fn const_boolean(
        &mut self,
        resolution: &dir::Call,
        index: usize,
        construct: &str,
    ) -> CompilerResult<bool> {
        let source = self.argument_expression(resolution, index)?;
        match self.node_type(source)? {
            dir::Type::Literal(dir::ScalarLiteral::Boolean(value)) => Ok(value),
            _ => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("{construct} without a const-evaluated value"),
            }
            .into()),
        }
    }

    /// Return one provided argument's value expression node.
    fn argument_expression(
        &mut self,
        resolution: &dir::Call,
        index: usize,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let arguments = &resolution.arguments;
        let Some(binding) = arguments.get(index) else {
            return Err(CompilerError::Internal {
                message: "too few arguments for one intrinsic".to_string(),
            });
        };
        let dir::ArgumentSource::Provided(argument) = binding.source else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a defaulted or spread intrinsic argument".to_string(),
            }
            .into());
        };
        let argument = argument.local_id.into_typed::<dir::Argument>();
        let Some(value) = self.source().tree().get(argument).value() else {
            return Err(CompilerError::Internal {
                message: "a valueless intrinsic argument".to_string(),
            });
        };

        Ok(value)
    }
}
