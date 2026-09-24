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

/// One intrinsic call being lowered: its selected call and the receiver evaluated ahead of it.
#[derive(Clone, Copy)]
pub(in crate::lower) struct IntrinsicCall<'call> {
    /// The call sema selected.
    pub(in crate::lower) resolution: &'call dir::Call,
    /// The receiver, argument 0 when present.
    pub(in crate::lower) receiver: Option<mir::Value>,
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
        let call = IntrinsicCall {
            resolution,
            receiver,
        };

        self.lower_named_intrinsic_call(expression, &name, call)
    }

    /// Lower one intrinsic call by its name through the intrinsic namespaces.
    fn lower_named_intrinsic_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        name: &str,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        // route the name through each intrinsic namespace in declaration order
        if let Ok(operation) = name.parse::<mir::Intrinsic>() {
            return self.lower_operation_intrinsic(operation, call);
        }
        if let Some(instruction) = IntrinsicInstruction::from_name(name) {
            return self.lower_instruction_intrinsic(expression, instruction, call);
        }
        if let Some(terminator) = IntrinsicTerminator::from_name(name) {
            return self.lower_terminator_intrinsic(terminator, call);
        }
        if let Some(layout) = LayoutIntrinsic::from_name(name) {
            return self.lower_layout_intrinsic(layout, call);
        }
        if let Some(context) = ContextIntrinsic::from_name(name) {
            return self.lower_context_intrinsic(context, call);
        }
        if matches!(name, "profile.counter" | "profile.sampler") {
            return self.lower_profile_instrument(expression, call);
        }

        Err(self.unsupported(format!("the '{name}' intrinsic")))
    }

    /// Lower one profile instrument definition to its zero-sized value.
    fn lower_profile_instrument(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        // evaluate the arguments for their effects
        for binding in &call.resolution.arguments {
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

    /// Return the instrument name one receiver's type names as its first type argument.
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
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        // lower the arguments without declared parameter representations
        let values = self.lower_call_arguments(&call.resolution.arguments, &[], &[])?;

        // emit a void operation and hand back no value
        if !operation.has_result() {
            self.builder.intrinsic_void(operation, values);

            return Ok(None);
        }

        // bind the operation result at the declared return type
        let result = self.lower_type(call.resolution.return_type)?;

        Ok(Some(self.builder.intrinsic(operation, result, values)))
    }

    /// Lower one instruction intrinsic.
    fn lower_instruction_intrinsic(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        instruction: IntrinsicInstruction,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        // dispatch on the instruction the name denotes
        match instruction {
            IntrinsicInstruction::AtomicFence => self.lower_atomic_fence(call),
            IntrinsicInstruction::AtomicLoad => self.lower_atomic_load(call),
            IntrinsicInstruction::AtomicCompareExchange { weak } => {
                self.lower_atomic_compare_exchange(weak, call)
            }
            IntrinsicInstruction::AtomicRmw(operator) => self.lower_atomic_rmw(operator, call),
            IntrinsicInstruction::AtomicStore => self.lower_atomic_store(call),
            IntrinsicInstruction::Breakpoint => {
                self.builder.breakpoint();

                Ok(None)
            }
            IntrinsicInstruction::Cast(operator) => self.lower_cast_intrinsic(operator, call),
            IntrinsicInstruction::PointerLoad => self.lower_pointer_load(call),
            IntrinsicInstruction::PointerStore => self.lower_pointer_store(call),
            IntrinsicInstruction::PointerReplace => self.lower_pointer_replace(call),
            IntrinsicInstruction::PointerSwap => self.lower_pointer_swap(call),
            IntrinsicInstruction::PointerDropInPlace => self.lower_pointer_drop_in_place(call),
            IntrinsicInstruction::Drop => {
                let value = self.argument_value(call, 0)?;
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
                self.lower_storage_constant(mir::Constant::Uninit, call)
            }
            IntrinsicInstruction::InitZeroed => {
                self.lower_storage_constant(mir::Constant::Zeroed, call)
            }
            IntrinsicInstruction::InitWrite => self.lower_init_write(call),
            IntrinsicInstruction::SliceFromRaw => self.lower_slice_from_raw(call),
            IntrinsicInstruction::SliceGet => self.lower_slice_get(call),
            IntrinsicInstruction::SliceLength => self.lower_slice_length(call),
            IntrinsicInstruction::SliceSet => self.lower_slice_set(call),
            IntrinsicInstruction::SliceView => self.lower_slice_view(call),
            IntrinsicInstruction::SliceUninit => self.lower_slice_uninit(call),
            IntrinsicInstruction::SliceIndex => self.lower_slice_index(call),
            IntrinsicInstruction::SliceAssumeInit => self.lower_slice_assume_init(call),
            // lower instrument intrinsics at their instrument method call sites
            IntrinsicInstruction::ProfileIncrement | IntrinsicInstruction::ProfileSample => {
                Err(self.internal("a profile instrument outside its call site"))
            }
            IntrinsicInstruction::Transmute => {
                let value = self.argument_value(call, 0)?;
                let result = self.lower_type(call.resolution.return_type)?;

                Ok(Some(self.builder.intrinsic(
                    mir::Intrinsic::Transmute,
                    result,
                    vec![value],
                )))
            }
            IntrinsicInstruction::Manage => {
                let value = self.argument_value(call, 0)?;
                let result = self.lower_type(call.resolution.return_type)?;

                Ok(Some(self.adopt(value, result)?))
            }
            IntrinsicInstruction::DynamicType => {
                let dynamic = self.argument_value(call, 0)?;

                Ok(Some(self.builder.dynamic_type(dynamic)))
            }
            IntrinsicInstruction::DynamicPayload => {
                let dynamic = self.argument_pointee_value(call, 0)?;
                let result = self.lower_type(call.resolution.return_type)?;

                Ok(Some(self.builder.dynamic_payload(dynamic, result)))
            }
            IntrinsicInstruction::VectorSplat => {
                let value = self.argument_value(call, 0)?;
                let vector = self.lower_type(call.resolution.return_type)?;

                Ok(Some(self.builder.vector_splat(vector, value)))
            }
            IntrinsicInstruction::VectorExtract => {
                let vector = self.argument_value(call, 0)?;
                let index = self.argument_value(call, 1)?;

                Ok(Some(self.builder.vector_extract(vector, index)))
            }
            IntrinsicInstruction::VectorInsert => {
                let vector = self.argument_value(call, 0)?;
                let index = self.argument_value(call, 1)?;
                let value = self.argument_value(call, 2)?;

                Ok(Some(self.builder.vector_insert(vector, index, value)))
            }
            IntrinsicInstruction::VectorSelect => {
                let mask = self.argument_value(call, 0)?;
                let then_value = self.argument_value(call, 1)?;
                let else_value = self.argument_value(call, 2)?;

                Ok(Some(
                    self.builder.vector_select(mask, then_value, else_value),
                ))
            }
            IntrinsicInstruction::VectorConvert => {
                let vector = self.argument_value(call, 0)?;
                let result = self.lower_type(call.resolution.return_type)?;

                Ok(Some(self.builder.vector_convert(
                    result,
                    mir::ConvertMode::Exact,
                    vector,
                )))
            }
            IntrinsicInstruction::VectorCompare(operator) => {
                let left = self.argument_value(call, 0)?;
                let right = self.argument_value(call, 1)?;
                let result = self.lower_type(call.resolution.return_type)?;

                Ok(Some(
                    self.builder.vector_compare(result, operator, left, right),
                ))
            }
            IntrinsicInstruction::VectorReduce(operator) => {
                let vector = self.argument_value(call, 0)?;

                Ok(Some(self.builder.vector_reduce(operator, vector)))
            }
        }
    }

    /// Lower one argument and read the value behind the reference it passes.
    fn argument_pointee_value(
        &mut self,
        call: IntrinsicCall<'_>,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        let reference = self.argument_value(call, index)?;
        let representation = self.value_representation(reference)?;
        match *self.builder.tree().type_definition(representation) {
            mir::Type::Reference { .. } => self.dereference(reference),
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
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        // emit the terminator the name denotes
        match terminator {
            IntrinsicTerminator::Abort => {
                let values = self.lower_call_arguments(&call.resolution.arguments, &[], &[])?;

                self.builder.abort(values.into_iter().next());
            }
            IntrinsicTerminator::Unreachable => self.builder.unreachable(),
            IntrinsicTerminator::Panic => {
                let values = self.lower_call_arguments(&call.resolution.arguments, &[], &[])?;

                self.builder.panic(values.into_iter().next());
            }
        }

        // continue building into an unreachable block after the terminator
        let dead = self.builder.block();
        self.builder.switch_to_block(dead);

        Ok(None)
    }

    /// Lower one atomic fence from its const ordering configuration.
    fn lower_atomic_fence(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let access = self.atomic_access(call, 0)?;
        self.builder.atomic_fence(mir::FenceAccess {
            ordering: access.ordering,
            scope: access.scope,
            storage: mir::StorageSet::ANY,
        });

        Ok(None)
    }

    /// Lower one atomic load from its pointer and const configuration.
    fn lower_atomic_load(&mut self, call: IntrinsicCall<'_>) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let access = self.atomic_access(call, 1)?;
        let result = self.lower_type(call.resolution.return_type)?;

        Ok(Some(self.builder.atomic_load(place, access, result)))
    }

    /// Lower one atomic store from its pointer, value, and configuration.
    fn lower_atomic_store(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let value = self.argument_value(call, 1)?;
        let access = self.atomic_access(call, 2)?;
        self.builder.atomic_store(place, value, access);

        Ok(None)
    }

    /// Lower one atomic read-modify-write returning the old value.
    fn lower_atomic_rmw(
        &mut self,
        operator: mir::AtomicRmwOperator,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let value = self.argument_value(call, 1)?;
        let access = self.atomic_access(call, 2)?;
        let result = self.lower_type(call.resolution.return_type)?;

        Ok(Some(
            self.builder
                .atomic_rmw(operator, place, value, access, result),
        ))
    }

    /// Lower one atomic compare-exchange returning the old value and success.
    fn lower_atomic_compare_exchange(
        &mut self,
        weak: bool,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let expected = self.argument_value(call, 1)?;
        let desired = self.argument_value(call, 2)?;
        let access = self.atomic_access(call, 3)?;
        let result = self.lower_type(call.resolution.return_type)?;

        Ok(Some(self.builder.atomic_compare_exchange(
            place,
            expected,
            desired,
            weak,
            mir::CompareExchangeAccess::with_success(access),
            result,
        )))
    }

    /// Read one atomic configuration starting at one argument position.
    fn atomic_access(
        &mut self,
        call: IntrinsicCall<'_>,
        start: usize,
    ) -> CompilerResult<mir::AtomicAccess> {
        // read the memory ordering, a const parameter by its index
        let source = self.argument_expression(call, start)?;
        let ordering = match self.read_parameter_index(source)? {
            Some(index) => mir::MemoryOrdering::Parameter(index),
            None => {
                let ordinal = self.enum_argument_ordinal(call, start, "an atomic ordering")?;
                let Some(ordering) = mir::MemoryOrdering::from_ordinal(ordinal) else {
                    return Err(self.internal(format!(
                        "an atomic ordering ordinal {ordinal} without a declared case"
                    )));
                };

                ordering
            }
        };

        // read the execution scope in the case order the language enum declares
        let ordinal = self.enum_argument_ordinal(call, start + 1, "an atomic scope")?;
        let Some(scope) = mir::ExecutionScope::from_ordinal(ordinal) else {
            return Err(self.internal(format!(
                "an atomic scope ordinal {ordinal} without a declared case"
            )));
        };

        // reject the three trailing flags
        for (offset, flag) in [
            (4, "a volatile atomic"),
            (5, "an availability-publishing atomic"),
            (6, "a visibility-acquiring atomic"),
        ] {
            if self.const_boolean(call, start + offset, flag)? {
                return Err(self.unsupported(flag.to_string()));
            }
        }

        Ok(mir::AtomicAccess::new(ordering, scope))
    }

    /// Lower one scalar cast of its sole operand.
    fn lower_cast_intrinsic(
        &mut self,
        operator: mir::CastOperator,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        // lower the operand the cast converts
        let source = self.argument_expression(call, 0)?;
        let operand = self.lower_value(source)?;

        // preserve the declared conversion until concrete emission
        let target = self.lower_type(call.resolution.return_type)?;

        Ok(Some(self.builder.cast(operator, operand, target)))
    }

    /// Lower one slice view over a raw address and length.
    fn lower_slice_from_raw(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let address = self.argument_value(call, 0)?;
        let length = self.argument_value(call, 1)?;
        let result = self.lower_type(call.resolution.return_type)?;

        Ok(Some(self.builder.aggregate(result, vec![address, length])))
    }

    /// Lower one slice element read.
    fn lower_slice_get(&mut self, call: IntrinsicCall<'_>) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(call, 0)?;
        let index = self.argument_value(call, 1)?;
        let element = self.lower_type(call.resolution.return_type)?;
        let place = mir::Place::value(slice)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Index { index });

        Ok(Some(self.load_place(place, element)))
    }

    /// Lower one slice element write.
    fn lower_slice_set(&mut self, call: IntrinsicCall<'_>) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(call, 0)?;
        let index = self.argument_value(call, 1)?;
        let value = self.argument_value(call, 2)?;
        let place = mir::Place::value(slice)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Index { index });
        self.builder.store(place, value);

        Ok(None)
    }

    /// Lower one slice length read.
    fn lower_slice_length(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(call, 0)?;

        Ok(Some(self.builder.slice_length(slice)))
    }

    /// Lower one slice view over a contiguous range.
    fn lower_slice_view(&mut self, call: IntrinsicCall<'_>) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(call, 0)?;
        let start = self.argument_value(call, 1)?;
        let length = self.argument_value(call, 2)?;
        let result = self.lower_type(call.resolution.return_type)?;

        let place = mir::Place::value(slice)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Slice { start, length });

        Ok(Some(self.builder.address(place, result)))
    }

    /// Lower one owned allocation of uninitialized slice elements.
    fn lower_slice_uninit(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let length = self.argument_value(call, 0)?;
        let result = self.lower_type(call.resolution.return_type)?;
        let mir::Type::Slice { element, .. } = self.builder.tree().get(result) else {
            return Err(CompilerError::Internal {
                message: "an uninitialized slice allocation outside a slice result".to_string(),
            });
        };
        let element = *element;
        let space = self.allocation_space(result)?;

        Ok(Some(
            self.builder
                .new_slice_uninit(element, length, result, space),
        ))
    }

    /// Lower one slice element borrow at the declared access.
    fn lower_slice_index(&mut self, call: IntrinsicCall<'_>) -> CompilerResult<Option<mir::Value>> {
        let slice = self.argument_value(call, 0)?;
        let index = self.argument_value(call, 1)?;
        let result = self.lower_type(call.resolution.return_type)?;

        let place = mir::Place::value(slice)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Index { index });

        Ok(Some(self.builder.address(place, result)))
    }

    /// Lower the completion of one owned slice with initialized elements.
    fn lower_slice_assume_init(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let storage = self.argument_value(call, 0)?;
        let result = self.lower_type(call.resolution.return_type)?;

        Ok(Some(self.builder.new_complete(storage, result)))
    }

    /// Lower one load through a raw pointer.
    fn lower_pointer_load(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let result = self.lower_type(call.resolution.return_type)?;

        // express the caller's unchecked read through the corresponding raw address
        let pointer = self.raw_pointer(pointer, result)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);

        Ok(Some(self.load_place(place, result)))
    }

    /// Lower one store through a raw pointer.
    fn lower_pointer_store(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let pointee = self.pointee_type(pointer)?;
        let pointer = self.raw_pointer(pointer, pointee)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let value = self.argument_value(call, 1)?;
        self.builder.store(place, value);

        Ok(None)
    }

    /// Lower one pointed-to value exchange returning the old value.
    fn lower_pointer_replace(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let result = self.lower_type(call.resolution.return_type)?;
        let pointer = self.raw_pointer(pointer, result)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let value = self.argument_value(call, 1)?;
        let old = self.load_place(place.clone(), result);
        self.builder.store(place, value);

        Ok(Some(old))
    }

    /// Lower one swap of two pointed-to values.
    fn lower_pointer_swap(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let first = self.argument_value(call, 0)?;
        let second = self.argument_value(call, 1)?;
        let pointee = self.pointee_type(first)?;
        let first = self.raw_pointer(first, pointee)?;
        let second = self.raw_pointer(second, pointee)?;
        let first = mir::Place::value(first).with_projection(mir::Projection::Deref);
        let second = mir::Place::value(second).with_projection(mir::Projection::Deref);
        let first_value = self.load_place(first.clone(), pointee);
        let second_value = self.load_place(second.clone(), pointee);
        self.builder.store(first, second_value);
        self.builder.store(second, first_value);

        Ok(None)
    }

    /// Lower one drop of the pointed-to value.
    fn lower_pointer_drop_in_place(
        &mut self,
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let pointee = self.pointee_type(pointer)?;
        let pointer = self.raw_pointer(pointer, pointee)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let value = self.load_place(place, pointee);
        self.builder.drop_value(value);

        Ok(None)
    }

    /// Reinterpret one pointer as the raw address its unchecked memory operations go through.
    fn raw_pointer(
        &mut self,
        pointer: mir::Value,
        pointee: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let source = self.value_representation(pointer)?;
        let target = match *self.builder.tree().type_definition(source) {
            mir::Type::Reference { access, .. } => mir::Type::Reference {
                kind: mir::Reference::Raw,
                lifetime: mir::Lifetime::empty(),
                access,
                pointee,
            },
            mir::Type::Pointer { access, .. } => mir::Type::Pointer { pointee, access },
            _ => return Err(self.internal("an unchecked memory operation without a pointer")),
        };
        let target = self.builder.tree_mut().intern_type(target);
        if source == target {
            return Ok(pointer);
        }

        Ok(self.builder.bitcast(pointer, target))
    }

    /// Return the pointee type behind one lowered pointer value.
    fn pointee_type(&mut self, pointer: mir::Value) -> CompilerResult<mir::TypeId> {
        let pointer_type = self.value_representation(pointer)?;
        let pointee = match self.builder.tree().type_definition(pointer_type) {
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
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let result = self.lower_type(call.resolution.return_type)?;

        Ok(Some(self.builder.constant(constant, result)))
    }

    /// Lower one initializing store returning the initialized borrow.
    fn lower_init_write(&mut self, call: IntrinsicCall<'_>) -> CompilerResult<Option<mir::Value>> {
        let pointer = self.argument_value(call, 0)?;
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let value = self.argument_value(call, 1)?;
        self.builder.store(place, value);
        let result = self.lower_type(call.resolution.return_type)?;

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
        call: IntrinsicCall<'_>,
    ) -> CompilerResult<Option<mir::Value>> {
        let value = match layout {
            // measure the subject size
            LayoutIntrinsic::SizeOf => {
                let ty = self.lower_type(call.resolution.return_type)?;
                self.subject_measure(call, mir::LayoutMeasure::Size, ty)?
            }
            // measure the subject alignment
            LayoutIntrinsic::AlignOf => {
                let ty = self.lower_type(call.resolution.return_type)?;
                self.subject_measure(call, mir::LayoutMeasure::Alignment, ty)?
            }
            // measure the padded element step
            LayoutIntrinsic::StrideOf => {
                let ty = self.lower_type(call.resolution.return_type)?;
                self.subject_measure(call, mir::LayoutMeasure::Stride, ty)?
            }
            // hand back the alignment itself as a well-aligned dangling address
            LayoutIntrinsic::Dangling => {
                let pointer = self.lower_type(call.resolution.return_type)?;
                let address = self.subject_measure(call, mir::LayoutMeasure::Alignment, pointer)?;

                self.builder
                    .intrinsic(mir::Intrinsic::Transmute, pointer, vec![address])
            }
            // move the pointer by an element count scaled to the stride
            LayoutIntrinsic::Offset => {
                let pointer = self.argument_value(call, 0)?;
                let count = self.argument_value(call, 1)?;
                let domain = self.value_representation(count)?;
                let step = self.subject_measure(call, mir::LayoutMeasure::Stride, domain)?;
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
                let result = self.lower_type(call.resolution.return_type)?;

                self.builder
                    .intrinsic(mir::Intrinsic::Transmute, result, vec![moved])
            }
            // divide the byte distance between two pointers back into elements
            LayoutIntrinsic::OffsetFrom => {
                let pointer = self.argument_value(call, 0)?;
                let origin = self.argument_value(call, 1)?;
                let domain = self.lower_type(call.resolution.return_type)?;
                let step = self.subject_measure(call, mir::LayoutMeasure::Stride, domain)?;
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

    /// Emit the layout measure resolved during instantiation.
    fn subject_measure(
        &mut self,
        call: IntrinsicCall<'_>,
        measure: mir::LayoutMeasure,
        ty: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let subject = self.subject_type(call)?;
        let constant = mir::Constant::Layout {
            ty: subject,
            measure,
        };

        Ok(self.builder.constant(constant, ty))
    }

    /// Return the lowered subject type argument of one layout call.
    fn subject_type(&mut self, call: IntrinsicCall<'_>) -> CompilerResult<mir::TypeId> {
        let dir::Call {
            target: dir::CallableTarget::Symbol { function, .. },
            ..
        } = call.resolution
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

    /// Lower one provided argument's value expression.
    pub(in crate::lower) fn argument_value(
        &mut self,
        call: IntrinsicCall<'_>,
        index: usize,
    ) -> CompilerResult<mir::Value> {
        if let (Some(receiver), 0) = (call.receiver, index) {
            return Ok(receiver);
        }
        let source = self.argument_expression(call, index)?;

        self.lower_value(source)
    }

    /// Read one enum-valued const argument as its declared case ordinal.
    fn enum_argument_ordinal(
        &mut self,
        call: IntrinsicCall<'_>,
        index: usize,
        construct: &str,
    ) -> CompilerResult<u32> {
        let source = self.argument_expression(call, index)?;
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
        call: IntrinsicCall<'_>,
        index: usize,
        construct: &str,
    ) -> CompilerResult<bool> {
        let source = self.argument_expression(call, index)?;
        match self.node_type(source)? {
            dir::Type::Literal(dir::Literal::Boolean(value)) => Ok(value),
            _ => Err(self.internal(format!("{construct} without a const-evaluated value"))),
        }
    }

    /// Return one provided argument's value expression node after the receiver.
    fn argument_expression(
        &mut self,
        call: IntrinsicCall<'_>,
        index: usize,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let index = match call.receiver {
            Some(_) if index == 0 => {
                return Err(CompilerError::Internal {
                    message: "an intrinsic reading its receiver as an expression".to_string(),
                });
            }
            Some(_) => index - 1,
            None => index,
        };
        let arguments = &call.resolution.arguments;
        let Some(binding) = arguments.get(index) else {
            let callee = match &call.resolution.target {
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
