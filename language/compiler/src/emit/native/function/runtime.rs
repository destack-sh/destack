use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::Module;
use destack_mir as mir;
use destack_native as native;
use destack_program::object::{FramePoint, Point};

use crate::EmitError;

use super::{Call, FunctionEmitter};

impl<'a> FunctionEmitter<'a> {
    /// Return the stack slot retaining one active platform unwind object.
    pub(super) fn unwind_slot(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> cir::StackSlot {
        if let Some(slot) = self.unwind {
            return slot;
        }

        // allocate one pointer-sized slot on first use
        let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
            cir::StackSlotKind::ExplicitSlot,
            self.types.pointer().bytes(),
            self.types.pointer().bytes().trailing_zeros() as u8,
        ));
        self.unwind = Some(slot);

        slot
    }

    /// Trap if one non-returning runtime operation returns unexpectedly.
    pub(super) fn terminate_runtime(builder: &mut cranelift_frontend::FunctionBuilder<'_>) {
        let trap = cir::TrapCode::unwrap_user(native::abi::Trap::Unreachable.code() as u8);
        builder.ins().trap(trap);
    }

    /// Poll runtime requests without crossing the runtime ABI on the common path.
    pub(super) fn emit_poll(
        &mut self,
        point: Point,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the pending request out of this activation
        let poll_request = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, poll_request),
            builder,
        )?;
        let poll_request =
            builder
                .ins()
                .atomic_load(cir::types::I32, cir::MemFlagsData::trusted(), poll_request);

        // branch to the cold block only when a request is set
        let slow = builder.create_block();
        let continuation = builder.create_block();
        builder
            .ins()
            .brif(poll_request, slow, &[], continuation, &[]);

        // capture the current frame only when runtime work is pending
        builder.switch_to_block(slow);
        builder.seal_block(slow);
        let frame = self.stack_map(FramePoint::operation(point.next()), builder)?;
        let id = self.frame_map_id(frame.id, builder)?;
        let call = self.emit_runtime(native::abi::Operation::Poll, &[id, frame.anchor], builder)?;
        Self::attach_stack_map(frame.entries, call, builder);
        Self::terminate_runtime(builder);

        // resume the operation stream after the cold runtime branch
        builder.switch_to_block(continuation);
        builder.seal_block(continuation);

        Ok(())
    }

    /// Return the heap space one managed reference type addresses.
    pub(super) fn heap_space(&self, ty: mir::TypeId) -> Result<mir::Space, EmitError> {
        // resolve the allocation storage type
        let ty = self.optimized.tree.storage_type(ty);
        let definition = self.optimized.tree.type_definition(ty);

        definition
            .reference_storage()
            .and_then(mir::Storage::heap_space)
            .ok_or_else(|| self.invalid("native reference does not address heap storage"))
    }

    /// Return the native projection of one heap space.
    pub(super) fn native_space(&self, space: mir::Space) -> Result<native::abi::Space, EmitError> {
        Ok(match space {
            mir::Space::Local => native::abi::Space::Local,
            mir::Space::Shared => native::abi::Space::Shared,
            mir::Space::Constant => {
                return Err(self.invalid("native allocation targets the constant image"));
            }
        })
    }

    /// Load one object-local identity resolved into a final Program index.
    pub(super) fn index_u32(
        &mut self,
        index: native::Index,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let address =
            self.index_address(index, size_of::<u32>(), align_of::<u32>() as u64, builder)?;
        let flags = cir::MemFlagsData::trusted();
        let value = builder.ins().load(cir::types::I32, flags, address, 0);

        Ok(value)
    }

    /// Load one object-local value resolved into a target-sized Program offset.
    pub(super) fn index_pointer(
        &mut self,
        index: native::Index,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let pointer = self.types.pointer();
        let byte_len = pointer.bytes() as usize;
        let address = self.index_address(index, byte_len, byte_len as u64, builder)?;
        let flags = cir::MemFlagsData::trusted();
        let value = builder.ins().load(pointer, flags, address, 0);

        Ok(value)
    }

    /// Return one function-local reference to a linked Program index.
    fn index_address(
        &mut self,
        index: native::Index,
        byte_len: usize,
        alignment: u64,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // declare the index symbol in the output object
        let symbol = self
            .symbols
            .declare(index, byte_len, alignment, self.output)
            .map_err(|error| Self::internal(self.module, error.to_string()))?;

        // reuse one Cranelift global value for every occurrence
        let reference = if let Some(reference) = self.indices.get(&index).copied() {
            reference
        } else {
            let reference = self.output.declare_data_in_func(symbol, builder.func);
            self.indices.insert(index, reference);

            reference
        };

        Ok(builder.ins().symbol_value(self.types.pointer(), reference))
    }

    /// Emit one runtime operation call.
    pub(super) fn emit_runtime(
        &mut self,
        operation: native::abi::Operation,
        arguments: &[cir::Value],
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Inst, EmitError> {
        let call = self.runtime_call(operation, arguments, builder)?;

        Ok(call.emit(builder))
    }

    /// Build one runtime-table call.
    pub(super) fn runtime_call(
        &mut self,
        operation: native::abi::Operation,
        arguments: &[cir::Value],
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<Call, EmitError> {
        // take the activation pointer ahead of the declared parameters
        let mut signature = cir::Signature::new(self.types.call_conv());
        signature
            .params
            .push(cir::AbiParam::new(self.types.pointer()));
        for parameter in operation.parameters() {
            let ty = self.runtime_type(*parameter);
            signature.params.push(cir::AbiParam::new(ty));
        }

        // require one argument of the declared type for each parameter
        let is_matching = signature.params.len() == arguments.len() + 1
            && signature.params[1..]
                .iter()
                .zip(arguments)
                .all(|(parameter, argument)| {
                    parameter.value_type == builder.func.dfg.value_type(*argument)
                });
        if !is_matching {
            return Err(self.invalid("native runtime arguments disagree with the operation"));
        }

        // return the operation's result type where it has one
        let result = match operation.result() {
            native::abi::OperationResult::Void | native::abi::OperationResult::Never => None,
            native::abi::OperationResult::Pointer => Some(native::abi::OperationValue::Pointer),
            native::abi::OperationResult::Uint32 => Some(native::abi::OperationValue::Uint32),
            native::abi::OperationResult::Uint64 => Some(native::abi::OperationValue::Uint64),
        };
        if let Some(result) = result {
            let ty = self.runtime_type(result);
            signature.returns.push(cir::AbiParam::new(ty));
        }

        // pass the activation through as the first argument
        let signature = builder.import_signature(signature);
        let mut parameters = vec![self.activation()?];
        parameters.extend_from_slice(arguments);

        // load the operation from this activation's runtime table
        let runtime = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, runtime),
            builder,
        )?;
        let function = builder.ins().load(
            self.types.pointer(),
            cir::MemFlagsData::trusted(),
            runtime,
            operation.offset() as i32,
        );

        Ok(Call::pointer(function, signature, parameters))
    }

    /// Return the Cranelift type of one runtime operation value.
    fn runtime_type(&self, value: native::abi::OperationValue) -> cir::Type {
        match value {
            native::abi::OperationValue::Pointer => self.types.pointer(),
            native::abi::OperationValue::Uint32 => cir::types::I32,
            native::abi::OperationValue::Uint64 => cir::types::I64,
        }
    }
}
