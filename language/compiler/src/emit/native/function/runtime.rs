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
        let activation = self.activation()?;
        let flags = cir::MemFlagsData::trusted();
        let poll_request = builder.ins().load(
            self.types.pointer(),
            flags,
            activation,
            std::mem::offset_of!(native::abi::Activation, poll_request) as i32,
        );
        let poll_request = builder
            .ins()
            .atomic_load(cir::types::I32, flags, poll_request);

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

    /// Return the heap space addressed by one reference-like type.
    pub(super) fn heap_space(&self, ty: mir::TypeId) -> Result<native::abi::Space, EmitError> {
        // resolve the allocation storage type
        let ty = self.optimized.tree.storage_type(ty);
        let definition = self.optimized.tree.get(ty);

        let space = definition
            .reference_storage()
            .and_then(mir::Storage::heap_space)
            .ok_or_else(|| self.invalid("native reference does not address heap storage"))?;

        Ok(match space {
            mir::Space::Local => native::abi::Space::Local,
            mir::Space::Shared => native::abi::Space::Shared,
            mir::Space::Constant | mir::Space::Parameter(_) => {
                return Err(self.invalid("native allocation targets a runtime space"));
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
        // take the activation pointer ahead of every declared argument
        let mut signature = cir::Signature::new(self.types.call_conv());
        signature
            .params
            .push(cir::AbiParam::new(self.types.pointer()));
        signature.params.extend(arguments.iter().map(|value| {
            let ty = builder.func.dfg.value_type(*value);

            cir::AbiParam::new(ty)
        }));

        // return the operation's result type where it has one
        let result = match operation.result() {
            native::abi::OperationResult::Void | native::abi::OperationResult::Never => None,
            native::abi::OperationResult::Pointer => Some(self.types.pointer()),
            native::abi::OperationResult::Uint32 => Some(cir::types::I32),
            native::abi::OperationResult::Uint64 => Some(cir::types::I64),
        };
        if let Some(result) = result {
            signature.returns.push(cir::AbiParam::new(result));
        }
        // pass the activation through as the first argument
        let signature = builder.import_signature(signature);
        let mut parameters = vec![self.activation()?];
        parameters.extend_from_slice(arguments);

        // load the operation from this activation's runtime table
        let flags = cir::MemFlagsData::trusted();
        let activation = self.activation()?;
        let runtime = builder.ins().load(
            self.types.pointer(),
            flags,
            activation,
            std::mem::offset_of!(native::abi::Activation, runtime) as i32,
        );
        let function = builder.ins().load(
            self.types.pointer(),
            flags,
            runtime,
            operation.offset() as i32,
        );

        Ok(Call::pointer(function, signature, parameters))
    }
}
