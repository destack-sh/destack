use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;
use destack_mir::{IntrinsicInstruction, IntrinsicTerminator};

use crate::lower::FunctionLowerer;
use crate::lower::function::context::ContextIntrinsic;
use crate::{CompilerError, CompilerResult};

/// One layout or pointer arithmetic intrinsic.
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
    /// Lower one intrinsic call through the callable's declared name.
    pub(in crate::lower) fn lower_intrinsic_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        name: Option<String>,
        resolution: &dir::Call,
        receiver: Option<mir::Value>,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(name) = name else {
            return Err(self.internal("an unnamed intrinsic callable"));
        };

        // read the receiver as argument 0 for the intrinsic's duration
        let enclosing = std::mem::replace(&mut self.intrinsic_receiver, receiver);
        let lowered = self.lower_named_intrinsic_call(expression, &name, resolution);
        self.intrinsic_receiver = enclosing;

        lowered
    }

    /// Lower one intrinsic call by its name through the intrinsic namespaces.
    fn lower_named_intrinsic_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        name: &str,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // route the name through each intrinsic namespace in declaration order
        if let Ok(operation) = name.parse::<mir::Intrinsic>() {
            return self.lower_operation_intrinsic(operation, resolution);
        }
        if let Some(instruction) = IntrinsicInstruction::from_name(name) {
            return self.lower_instruction_intrinsic(expression, instruction, resolution);
        }
        if let Some(terminator) = IntrinsicTerminator::from_name(name) {
            return self.lower_terminator_intrinsic(terminator, resolution);
        }
        if let Some(layout) = LayoutIntrinsic::from_name(name) {
            return self.lower_layout_intrinsic(layout, resolution);
        }
        if let Some(context) = ContextIntrinsic::from_name(name) {
            return self.lower_context_intrinsic(context, resolution);
        }
        if matches!(name, "profile.counter" | "profile.sampler") {
            return self.lower_profile_instrument(expression, resolution);
        }

        Err(self.unsupported(format!("the '{name}' intrinsic")))
    }

    /// Lower one profile instrument definition to its zero-sized value.
    fn lower_profile_instrument(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // evaluate the arguments for their effects
        for binding in &resolution.arguments {
            if let dir::ArgumentSource::Provided(argument) = binding.source {
                self.lower_argument(argument)?;
            }
        }
        let instrument = self.lower_type(self.node_type_id(expression)?)?;

        Ok(Some(
            self.builder.constant(mir::Constant::Zeroed, instrument),
        ))
    }

    /// Lower one instrument method call at its site, named by the receiver's type.
    pub(in crate::lower) fn lower_profile_call(
        &mut self,
        resolution: &dir::Call,
        item: dir::LanguageItem,
    ) -> CompilerResult<Option<mir::Value>> {
        // read the instrument name off the receiver's type argument
        let dir::CallableTarget::Symbol { function, .. } = &resolution.target else {
            return Err(CompilerError::Internal {
                message: "a profile call outside a selected function".to_string(),
            });
        };
        let Some(receiver) = &function.receiver else {
            return Err(self.internal("a profile instrument outside its member call"));
        };
        let name = self.profile_instrument_name(receiver.source)?;

        // read or write the instrument the item names at that site
        match item {
            dir::LanguageItem::ProfileCounterIncrement => {
                let site = mir::CounterSite::Named(name);
                let counter = match self.profile.counter(&site) {
                    Some(counter) => counter,
                    None => self.profile.insert_counter(site),
                };
                self.builder.profile_increment(counter);
            }
            dir::LanguageItem::ProfileSamplerSample => {
                let Some(dir::ArgumentSource::Provided(value)) =
                    resolution.arguments.first().map(|binding| &binding.source)
                else {
                    return Err(CompilerError::Internal {
                        message: "a profile sample without its value argument".to_string(),
                    });
                };
                let value = self.lower_argument(*value)?;
                let site = mir::SampleSite::Named(name);
                let sampler = match self.profile.sampler(&site) {
                    Some(sampler) => sampler,
                    None => self.profile.insert_sampler(site),
                };
                self.builder.profile_sample(sampler, value);
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "a profile call outside the instrument methods".to_string(),
                });
            }
        }

        Ok(None)
    }

    /// Return the instrument name one receiver's type carries as its first type argument.
    fn profile_instrument_name(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<StringId> {
        let mut ty = self.lower.stored(ty)?;
        while let Some(layer) = self.lower.indirection(ty, &self.scope)? {
            // stop at the implicit managed layer a bare reference family stores itself under
            if layer.stored == ty {
                break;
            }
            ty = layer.stored;
        }
        let dir::Type::Application(application) = self.lower.ty(ty)? else {
            return Err(CompilerError::Internal {
                message: "a profile instrument outside its applied type".to_string(),
            });
        };
        let name = self
            .lower
            .types(ty.module_id)?
            .type_ids(application.arguments)
            .first()
            .copied();
        match name.map(|name| self.lower.ty(name)).transpose()? {
            Some(dir::Type::Literal(dir::Literal::String(name))) => Ok(name),
            _ => Err(CompilerError::Internal {
                message: "a profile instrument without its literal name".to_string(),
            }),
        }
    }

    /// Lower one operation intrinsic.
    fn lower_operation_intrinsic(
        &mut self,
        operation: mir::Intrinsic,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // lower the arguments without declared parameter representations
        let values = self.lower_call_arguments(&resolution.arguments, &[], None)?;

        // emit a void operation and hand back no value
        if !operation.has_result() {
            self.builder.intrinsic_void(operation, values);

            return Ok(None);
        }

        // bind the operation result at the declared return type
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.intrinsic(operation, result, values)))
    }

    /// Lower one instruction intrinsic.
    fn lower_instruction_intrinsic(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        instruction: IntrinsicInstruction,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // dispatch on the instruction the name denotes
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
                let dropped = self.builder.drop_value(value);

                // anchor the authored drop call at its written extent
                let Some(span) = self.source().tree().get_source_extent(expression) else {
                    return Err(CompilerError::Internal {
                        message: "a drop call without a source extent".to_string(),
                    });
                };
                self.builder.tree_mut().set_span(dropped, span);

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
            IntrinsicInstruction::SliceUninit => self.lower_slice_uninit(resolution),
            IntrinsicInstruction::SliceIndex => self.lower_slice_index(resolution),
            IntrinsicInstruction::SliceAssumeInit => self.lower_slice_assume_init(resolution),
            // lower instrument intrinsics at their instrument method call sites
            IntrinsicInstruction::ProfileIncrement | IntrinsicInstruction::ProfileSample => {
                Err(self.internal("a profile instrument outside its call site"))
            }
            IntrinsicInstruction::Transmute => {
                let value = self.argument_value(resolution, 0)?;
                let result = self.lower_type(resolution.return_type)?;

                Ok(Some(self.builder.intrinsic(
                    mir::Intrinsic::Transmute,
                    result,
                    vec![value],
                )))
            }
            IntrinsicInstruction::Manage => {
                let value = self.argument_value(resolution, 0)?;
                let result = self.lower_type(resolution.return_type)?;

                Ok(Some(self.adopt(value, result)?))
            }
            IntrinsicInstruction::DynamicType => {
                let dynamic = self.argument_value(resolution, 0)?;

                Ok(Some(self.builder.dynamic_type(dynamic)))
            }
            IntrinsicInstruction::DynamicPayload => {
                let dynamic = self.argument_pointee_value(resolution, 0)?;
                let result = self.lower_type(resolution.return_type)?;

                Ok(Some(self.builder.dynamic_payload(dynamic, result)))
            }
            IntrinsicInstruction::VectorSplat => {
                let value = self.argument_value(resolution, 0)?;
                let vector = self.lower_type(resolution.return_type)?;

                Ok(Some(self.builder.vector_splat(vector, value)))
            }
            IntrinsicInstruction::VectorExtract => {
                let vector = self.argument_value(resolution, 0)?;
                let index = self.argument_value(resolution, 1)?;

                Ok(Some(self.builder.vector_extract(vector, index)))
            }
            IntrinsicInstruction::VectorInsert => {
                let vector = self.argument_value(resolution, 0)?;
                let index = self.argument_value(resolution, 1)?;
                let value = self.argument_value(resolution, 2)?;

                Ok(Some(self.builder.vector_insert(vector, index, value)))
            }
            IntrinsicInstruction::VectorSelect => {
                let mask = self.argument_value(resolution, 0)?;
                let then_value = self.argument_value(resolution, 1)?;
                let else_value = self.argument_value(resolution, 2)?;

                Ok(Some(
                    self.builder.vector_select(mask, then_value, else_value),
                ))
            }
            IntrinsicInstruction::VectorConvert => {
                let vector = self.argument_value(resolution, 0)?;
                let result = self.lower_type(resolution.return_type)?;

                Ok(Some(self.builder.vector_convert(
                    result,
                    mir::ConvertMode::Exact,
                    vector,
                )))
            }
            IntrinsicInstruction::VectorCompare(operator) => {
                let left = self.argument_value(resolution, 0)?;
                let right = self.argument_value(resolution, 1)?;
                let result = self.lower_type(resolution.return_type)?;

                Ok(Some(
                    self.builder.vector_compare(result, operator, left, right),
                ))
            }
            IntrinsicInstruction::VectorReduce(operator) => {
                let vector = self.argument_value(resolution, 0)?;

                Ok(Some(self.builder.vector_reduce(operator, vector)))
            }
        }
    }

    /// Lower one argument and read the value behind the reference it passes, a fat reference
    /// family value standing for itself.
    fn argument_pointee_value(
        &mut self,
        resolution: &dir::Call,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        let reference = self.argument_value(resolution, index)?;
        let representation = self.value_representation(reference)?;
        match *self.builder.tree().get(representation) {
            mir::Type::Reference { pointee, .. } => Ok(self.builder.load(reference, pointee)),
            mir::Type::Dynamic { .. } | mir::Type::Slice { .. } | mir::Type::Function { .. } => {
                Ok(reference)
            }
            _ => Err(CompilerError::Internal {
                message: "an intrinsic reading through a value outside a reference".to_string(),
            }),
        }
    }

    /// Lower one terminator intrinsic and open a dead block for trailing code.
    fn lower_terminator_intrinsic(
        &mut self,
        terminator: IntrinsicTerminator,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // emit the terminator the name denotes
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
    /// The fence orders every storage region, whatever the call narrows it to.
    fn lower_atomic_fence(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let access = self.atomic_access(resolution, 0)?;
        self.builder.atomic_fence(mir::FenceAccess {
            ordering: access.ordering,
            scope: access.scope,
            storage: mir::StorageSet::ANY,
        });

        Ok(None)
    }

    /// Lower one atomic load from its pointer and const configuration.
    fn lower_atomic_load(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let access = self.atomic_access(resolution, 1)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.atomic_load(pointer, access, result)))
    }

    /// Lower one atomic store from its pointer, value, and configuration.
    fn lower_atomic_store(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let value = self.argument_value(resolution, 1)?;
        let access = self.atomic_access(resolution, 2)?;
        self.builder.atomic_store(pointer, value, access);

        Ok(None)
    }

    /// Lower one atomic read-modify-write returning the old value.
    fn lower_atomic_rmw(
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
    /// The failed comparison loads at the success ordering stripped of its release half.
    fn lower_atomic_compare_exchange(
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

    /// Read one atomic configuration starting at one argument position.
    fn atomic_access(
        &mut self,
        resolution: &dir::Call,
        start: usize,
    ) -> CompilerResult<mir::AtomicAccess> {
        // read the ordering in the case order the language enum declares
        let ordering = match self.enum_argument_ordinal(resolution, start, "an atomic ordering")? {
            0 => mir::MemoryOrdering::Relaxed,
            1 => mir::MemoryOrdering::Acquire,
            2 => mir::MemoryOrdering::Release,
            3 => mir::MemoryOrdering::AcquireRelease,
            4 => mir::MemoryOrdering::SequentiallyConsistent,
            other => {
                return Err(
                    self.internal(format!("the atomic ordering in declaration order {other}"))
                );
            }
        };

        // read the execution scope in the case order the language enum declares
        let scope = match self.enum_argument_ordinal(resolution, start + 1, "an atomic scope")? {
            0 => mir::ExecutionScope::Invocation,
            1 => mir::ExecutionScope::Subgroup,
            2 => mir::ExecutionScope::Workgroup,
            3 => mir::ExecutionScope::Device,
            7 => mir::ExecutionScope::System,
            other => {
                return Err(self.internal(format!("the atomic scope in declaration order {other}")));
            }
        };

        // reject the three trailing flags
        for (offset, flag) in [
            (4, "a volatile atomic"),
            (5, "an availability-publishing atomic"),
            (6, "a visibility-acquiring atomic"),
        ] {
            if self.const_boolean(resolution, start + offset, flag)? {
                return Err(self.unsupported(flag.to_string()));
            }
        }

        Ok(mir::AtomicAccess::new(ordering, scope))
    }

    /// Lower one scalar cast of its sole operand.
    fn lower_cast_intrinsic(
        &mut self,
        operator: mir::CastOperator,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // lower the operand the cast converts
        let source = self.argument_expression(resolution, 0)?;
        let operand = self.lower_value(source)?;

        // read both scalar formats, a parameter keeping the cast as written
        let source_type = self.node_type_id(source)?;
        let source_type = self.lower.ty(source_type)?;
        let target_type = self.lower.ty(resolution.return_type)?;
        if matches!(source_type, dir::Type::Parameter(_))
            || matches!(target_type, dir::Type::Parameter(_))
        {
            let target = self.lower_type(resolution.return_type)?;
            let source = self.value_representation(operand)?;
            if source == target {
                return Ok(Some(operand));
            }

            return Ok(Some(self.builder.cast(operator, operand, target)));
        }
        let source_format = self.lower.scalar_type(&source_type)?;
        let target_format = self.lower.scalar_type(&target_type)?;

        // hand the value through unchanged when it already carries the target
        if source_format == target_format {
            return Ok(Some(operand));
        }

        // refine the declared conversion intent to the instantiated formats
        let operator = match operator {
            mir::CastOperator::Truncate
            | mir::CastOperator::FloatToSignedIntSaturating
            | mir::CastOperator::FloatToUnsignedIntSaturating => {
                self.cast_operator(&source_format, &target_format)?
            }
            operator => operator,
        };
        let target = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.cast(operator, operand, target)))
    }

    /// Lower one slice view over a raw address and length.
    fn lower_slice_from_raw(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let address = self.argument_value(resolution, 0)?;
        let length = self.argument_value(resolution, 1)?;
        let start = self.builder.iconst(0, 64, false);
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(
            self.builder.slice_view(address, start, length, result),
        ))
    }

    /// Lower one slice element read.
    fn lower_slice_get(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;
        let index = self.argument_value(resolution, 1)?;
        let element = self.lower_type(resolution.return_type)?;
        let pointer = self.element_address(slice, index, element)?;

        Ok(Some(self.builder.load(pointer, element)))
    }

    /// Lower one slice element write.
    fn lower_slice_set(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;
        let index = self.argument_value(resolution, 1)?;
        let value = self.argument_value(resolution, 2)?;
        let element = self.value_representation(value)?;
        let pointer = self.element_address(slice, index, element)?;
        self.builder.store(pointer, value);

        Ok(None)
    }

    /// Lower one slice length read.
    fn lower_slice_length(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;

        Ok(Some(self.builder.slice_length(slice)))
    }

    /// Lower one slice view over a contiguous range.
    fn lower_slice_view(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;
        let start = self.argument_value(resolution, 1)?;
        let length = self.argument_value(resolution, 2)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.slice_view(slice, start, length, result)))
    }

    /// Lower one owned allocation of uninitialized slice elements.
    fn lower_slice_uninit(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let length = self.argument_value(resolution, 0)?;
        let result = self.lower_type(resolution.return_type)?;
        let mir::Type::Slice { element, .. } = self.builder.tree().get(result) else {
            return Err(CompilerError::Internal {
                message: "an uninitialized slice allocation outside a slice result".to_string(),
            });
        };
        let element = *element;

        Ok(Some(self.builder.new_slice_uninit(element, length, result)))
    }

    /// Lower one slice element borrow at the declared access.
    fn lower_slice_index(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(resolution, 0)?;
        let index = self.argument_value(resolution, 1)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.element_addr(
            slice,
            index,
            result,
            mir::AddressKind::Borrow,
        )))
    }

    /// Lower the completion of one owned slice with initialized elements.
    fn lower_slice_assume_init(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let storage = self.argument_value(resolution, 0)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.new_complete(storage, result)))
    }

    /// Address one slice element behind an exclusive borrow representation.
    fn element_address(
        &mut self,
        slice: mir::Value,
        index: mir::Value,
        element: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // address the element at the slice's own storage and access
        let representation = self.value_representation(slice)?;
        let (storage, access) = match *self.builder.tree().get(representation) {
            mir::Type::Slice {
                storage, access, ..
            } => (storage, access),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a slice value without slice storage".to_string(),
                });
            }
        };
        let lifetime = self.reborrow_lifetime(slice);
        let pointer = self.insert_reference(
            mir::ReferenceKind::Borrowed,
            lifetime,
            access,
            storage,
            element,
        );

        Ok(self
            .builder
            .element_addr(slice, index, pointer, mir::AddressKind::Borrow))
    }

    /// Lower one load through a raw pointer.
    fn lower_pointer_load(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let pointer = self.raw_pointer(pointer)?;
        let result = self.lower_type(resolution.return_type)?;

        Ok(Some(self.builder.load(pointer, result)))
    }

    /// Lower one store through a raw pointer.
    fn lower_pointer_store(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let pointer = self.raw_pointer(pointer)?;
        let value = self.argument_value(resolution, 1)?;
        self.builder.store(pointer, value);

        Ok(None)
    }

    /// Lower one pointed-to value exchange returning the old value.
    fn lower_pointer_replace(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(resolution, 0)?;
        let pointer = self.raw_pointer(pointer)?;
        let value = self.argument_value(resolution, 1)?;
        let result = self.lower_type(resolution.return_type)?;
        let old = self.builder.load(pointer, result);
        self.builder.store(pointer, value);

        Ok(Some(old))
    }

    /// Return one reference or pointer as a raw pointer.
    fn raw_pointer(&mut self, value: mir::Value) -> CompilerResult<mir::Value> {
        let ty = self.value_representation(value)?;
        let mir::Type::Reference {
            pointee, access, ..
        } = *self.builder.tree().get(ty)
        else {
            return Ok(value);
        };
        let pointer = self
            .builder
            .tree_mut()
            .intern_type(mir::Type::Pointer { pointee, access });

        Ok(self
            .builder
            .cast(mir::CastOperator::Bitcast, value, pointer))
    }

    /// Lower one swap of two pointed-to values.
    fn lower_pointer_swap(&mut self, resolution: &dir::Call) -> CompilerResult<Option<mir::Value>> {
        let first = self.argument_value(resolution, 0)?;
        let first = self.raw_pointer(first)?;
        let second = self.argument_value(resolution, 1)?;
        let second = self.raw_pointer(second)?;
        let pointee = self.pointee_type(first)?;
        let first_value = self.builder.load(first, pointee);
        let second_value = self.builder.load(second, pointee);
        self.builder.store(first, second_value);
        self.builder.store(second, first_value);

        Ok(None)
    }

    /// Lower one drop of the pointed-to value.
    fn lower_pointer_drop_in_place(
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
        let pointer_type = self.value_representation(pointer)?;
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

    /// Lower one layout intrinsic to its folded target constants.
    fn lower_layout_intrinsic(
        &mut self,
        layout: LayoutIntrinsic,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let value = match layout {
            // measure the subject size
            LayoutIntrinsic::SizeOf => {
                let ty = self.lower_type(resolution.return_type)?;
                self.subject_measure(resolution, mir::LayoutMeasure::Size, ty)?
            }
            // measure the subject alignment
            LayoutIntrinsic::AlignOf => {
                let ty = self.lower_type(resolution.return_type)?;
                self.subject_measure(resolution, mir::LayoutMeasure::Alignment, ty)?
            }
            // measure the padded element step
            LayoutIntrinsic::StrideOf => {
                let ty = self.lower_type(resolution.return_type)?;
                self.subject_measure(resolution, mir::LayoutMeasure::Stride, ty)?
            }
            // hand back the alignment itself as a well-aligned dangling address
            LayoutIntrinsic::Dangling => {
                let pointer = self.lower_type(resolution.return_type)?;
                let address =
                    self.subject_measure(resolution, mir::LayoutMeasure::Alignment, pointer)?;

                self.builder
                    .intrinsic(mir::Intrinsic::Transmute, pointer, vec![address])
            }
            // move the pointer by an element count scaled to the stride
            LayoutIntrinsic::Offset => {
                let pointer = self.argument_value(resolution, 0)?;
                let count = self.argument_value(resolution, 1)?;
                let domain = self.value_representation(count)?;
                let step = self.subject_measure(resolution, mir::LayoutMeasure::Stride, domain)?;
                let domain = self.value_representation(step)?;
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
            // divide the byte distance between two pointers back into elements
            LayoutIntrinsic::OffsetFrom => {
                let pointer = self.argument_value(resolution, 0)?;
                let origin = self.argument_value(resolution, 1)?;
                let domain = self.lower_type(resolution.return_type)?;
                let step = self.subject_measure(resolution, mir::LayoutMeasure::Stride, domain)?;
                let domain = self.value_representation(step)?;
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

    /// Measure one call's subject type argument, folded unless the subject mentions a parameter.
    fn subject_measure(
        &mut self,
        resolution: &dir::Call,
        measure: mir::LayoutMeasure,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        let subject = self.subject_type(resolution)?;
        if mir::TypeId::from(subject).mentions_parameter(self.builder.tree()) {
            let constant = mir::Constant::Layout {
                ty: mir::TypeId::from(subject),
                measure,
            };
            return Ok(self.builder.constant(constant, ty));
        }
        let layout = self.subject_layout(resolution)?;
        let value = match measure {
            mir::LayoutMeasure::Size => layout.size,
            mir::LayoutMeasure::Alignment => layout.alignment.max(1),
            mir::LayoutMeasure::Stride => layout.size.next_multiple_of(layout.alignment.max(1)),
        };
        Ok(match self.builder.tree().get(ty).clone() {
            mir::Type::Isize => self.builder.isize_const(value as i128),
            mir::Type::Int { width, is_signed } => {
                self.builder.iconst(value as i128, width, is_signed)
            }
            _ => self.builder.usize_const(value as u128),
        })
    }

    /// Return the lowered subject type argument of one layout call.
    fn subject_type(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let dir::Call {
            target: dir::CallableTarget::Symbol { function, .. },
            ..
        } = resolution
        else {
            return Err(CompilerError::Internal {
                message: "a layout intrinsic without a candidate".to_string(),
            });
        };
        let bindings = self.lower.selection_bindings(&function.key)?;
        let Some(subject) = bindings.first().map(|binding| binding.argument) else {
            return Err(CompilerError::Internal {
                message: "a layout intrinsic without a subject type".to_string(),
            });
        };

        self.lower_type(subject)
    }

    /// Return the folded layout of one call's subject type argument.
    fn subject_layout(&mut self, resolution: &dir::Call) -> CompilerResult<mir::Layout> {
        let subject = self.subject_type(resolution)?;

        // lay the subject out at the target to answer the query
        let pointer_bytes = (self.builder.pointer_bits() / 8) as u8;
        let target = mir::TargetLayout::for_pointer_bytes(pointer_bytes);
        let mut layouts = mir::LayoutTable::default();
        let mut builder = mir::LayoutBuilder::new(self.builder.tree_mut(), &mut layouts, target);
        let id = builder
            .layout_type(subject)
            .map_err(|error| CompilerError::from((self.lower.module, error)))?;

        Ok(layouts.layout(id).clone())
    }

    /// Lower one provided argument's value expression.
    pub(in crate::lower) fn argument_value(
        &mut self,
        resolution: &dir::Call,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        if let (Some(receiver), 0) = (self.intrinsic_receiver, index) {
            return Ok(receiver);
        }
        let source = self.argument_expression(resolution, index)?;

        self.lower_value(source)
    }

    /// Read one enum-valued const argument as its declared case ordinal.
    fn enum_argument_ordinal(
        &mut self,
        resolution: &dir::Call,
        index: usize,
        construct: &str,
    ) -> CompilerResult<u32> {
        let source = self.argument_expression(resolution, index)?;
        let dir::Type::Variant(member) = self.node_type(source)? else {
            return Err(self.internal(format!("{construct} without a const-evaluated value")));
        };
        let dir::Type::Application(owner) = self.lower.ty(member.owner)? else {
            return Err(CompilerError::Internal {
                message: "an enum member without its owner instance".to_string(),
            });
        };

        self.lower.variant_position(owner.symbol, member.variant)
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
            dir::Type::Literal(dir::Literal::Boolean(value)) => Ok(value),
            _ => Err(self.internal(format!("{construct} without a const-evaluated value"))),
        }
    }

    /// Return one provided argument's value expression node, argument 0 following a method-form
    /// intrinsic's receiver.
    fn argument_expression(
        &mut self,
        resolution: &dir::Call,
        index: usize,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let index = match self.intrinsic_receiver {
            Some(_) if index == 0 => {
                return Err(CompilerError::Internal {
                    message: "an intrinsic reading its receiver as an expression".to_string(),
                });
            }
            Some(_) => index - 1,
            None => index,
        };
        let arguments = &resolution.arguments;
        let Some(binding) = arguments.get(index) else {
            let callee = match &resolution.target {
                dir::CallableTarget::Symbol { function, .. } => {
                    self.lower.symbol_path(function.key.symbol)?
                }
                _ => "a dispatched callee".to_string(),
            };
            let anchor = self
                .builder
                .source()
                .map(|(_, span)| format!(" at {span:?}"))
                .unwrap_or_default();
            return Err(CompilerError::Internal {
                message: format!(
                    "an intrinsic call of '{callee}' with {} arguments reading argument {index}{anchor}",
                    arguments.len()
                ),
            });
        };
        let dir::ArgumentSource::Provided(argument) = binding.source else {
            return Err(self.internal("a defaulted or spread intrinsic argument"));
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
