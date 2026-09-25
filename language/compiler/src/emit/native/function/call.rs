use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::Module;
use tspp_mir as mir;
use tspp_native as native;
use tspp_program as program;

use crate::EmitError;

use super::super::r#type::ValueType;
use super::{FrameMap, FunctionEmitter, Value};

/// One physical native call.
pub(super) struct Call {
    /// Callable target.
    pub(super) target: Target,
    /// Physical calling signature.
    pub(super) signature: cir::SigRef,
    /// Flattened call arguments.
    pub(super) arguments: Vec<cir::Value>,
    /// Logical result delivered to the MIR continuation.
    pub(super) result: Return,
}

/// Callable native target.
pub(super) enum Target {
    /// Direct object function.
    Function(cir::FuncRef),
    /// Resolved indirect native body.
    Pointer(cir::Value),
}

/// Logical call result delivered after physical return.
#[derive(Clone, Copy)]
pub(super) enum Return {
    /// No call result.
    Void,
    /// Result returned in native registers.
    Registers(ValueType),
    /// Indirect native result written at one canonical address.
    Address {
        /// Result address passed to the callee.
        address: cir::Value,
    },
    /// Canonical Word buffer written by a runtime binding.
    Buffer {
        /// Result buffer passed to the runtime.
        address: cir::Value,
        /// Result value type.
        value_type: ValueType,
    },
}

impl Call {
    /// Create one indirect native call without a logical result.
    pub(super) fn pointer(
        function: cir::Value,
        signature: cir::SigRef,
        arguments: Vec<cir::Value>,
    ) -> Self {
        Self {
            target: Target::Pointer(function),
            signature,
            arguments,
            result: Return::Void,
        }
    }

    /// Emit this call as one ordinary native instruction.
    pub(super) fn emit(&self, builder: &mut cranelift_frontend::FunctionBuilder<'_>) -> cir::Inst {
        match self.target {
            Target::Function(function) => builder.ins().call(function, &self.arguments),
            Target::Pointer(function) => {
                builder
                    .ins()
                    .call_indirect(self.signature, function, &self.arguments)
            }
        }
    }

    /// Emit this call with explicit normal and exceptional continuations.
    pub(super) fn invoke(
        &self,
        exceptions: cir::ExceptionTable,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> cir::Inst {
        match self.target {
            Target::Function(function) => {
                builder
                    .ins()
                    .try_call(function, &self.arguments, exceptions)
            }
            Target::Pointer(function) => {
                builder
                    .ins()
                    .try_call_indirect(function, &self.arguments, exceptions)
            }
        }
    }
}

impl FunctionEmitter<'_> {
    /// Emit one direct call and retain its result.
    pub(super) fn emit_call(
        &mut self,
        destination: Option<mir::Value>,
        call: &mir::Call,
        frame: FrameMap,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Inst, EmitError> {
        if self.is_imported_binding(call)? {
            let (call_instruction, result) = self.emit_binding_values(call, builder)?;
            Self::attach_stack_map(frame.entries, call_instruction, builder);
            if let Some(destination) = destination {
                let (address, value_type) = result
                    .ok_or_else(|| self.invalid("native binding did not retain its result"))?;
                let result = self.load(address, value_type, builder)?;
                self.set(destination, result)?;
            }

            return Ok(call_instruction);
        }

        let (call_instruction, indirect_result) = self.emit_call_values(call, &frame, builder)?;
        let results = builder.inst_results(call_instruction).to_vec();
        Self::attach_stack_map(frame.entries, call_instruction, builder);
        if let Some(destination) = destination {
            let value_type = self.types.value(self.value_type(destination)?)?;
            let value = match value_type {
                ValueType::Direct { .. } => Value::Direct(results[0]),
                ValueType::ScalarPair { .. } => Value::ScalarPair([results[0], results[1]]),
                ValueType::Indirect { .. } => {
                    let address = indirect_result.ok_or_else(|| {
                        self.invalid("native call did not retain its indirect result")
                    })?;

                    Value::Address(address)
                }
            };
            self.set(destination, value)?;
        }

        Ok(call_instruction)
    }

    /// Emit one direct internal call.
    pub(super) fn emit_call_values(
        &mut self,
        call: &mir::Call,
        frame: &FrameMap,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(cir::Inst, Option<cir::Value>), EmitError> {
        if self.is_imported_binding(call)? {
            return Err(self.invalid("native tail calls cannot target runtime bindings"));
        }

        let call = self.call(call, frame, builder)?;
        let indirect_result = match call.result {
            Return::Address { address } => Some(address),
            Return::Void | Return::Registers(_) | Return::Buffer { .. } => None,
        };
        let instruction = call.emit(builder);

        Ok((instruction, indirect_result))
    }

    /// Build one internal native call.
    pub(super) fn call(
        &mut self,
        call: &mir::Call,
        frame: &FrameMap,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<Call, EmitError> {
        // emit imported bindings through the binding call path
        if self.is_imported_binding(call)? {
            return self.binding_call(call, builder);
        }

        let result = self
            .optimized
            .tree
            .get(call.signature)
            .function_signature_parts()
            .map(|(_, _, result)| result)
            .ok_or_else(|| self.invalid("native call has no callable signature"))?;
        let result_type = self.types.result(result)?;
        let mut arguments = vec![self.activation()?];

        // allocate canonical storage for an indirect call result
        let result = if let Some(result_type) = result_type {
            if result_type.is_indirect() {
                let address = self.allocate(result_type, builder);
                arguments.push(address);

                Return::Address { address }
            } else {
                Return::Registers(result_type)
            }
        } else {
            Return::Void
        };
        let (direct, identity, environment) = match call.callee {
            mir::Callee::Witness { .. } => {
                return Err(self.invalid("witness calls resolve at instantiation"));
            }
            // resolve direct calls through the object module
            mir::Callee::Direct { function, .. } => {
                let function_id = self
                    .functions
                    .get(&function)
                    .copied()
                    .ok_or_else(|| self.invalid("native callee was not declared"))?;
                let callee = self.optimized.tree.get(function);
                if callee.environment.is_some() {
                    return Err(self
                        .invalid("direct call to captured native function lacks an environment"));
                }
                let function = self.output.declare_func_in_func(function_id, builder.func);
                builder.func.dfg.ext_funcs[function].colocated = true;

                (Some(function), None, None)
            }
            // unpack function identities and closure environments directly from SSA
            mir::Callee::Indirect { value } => match self.value(value)? {
                Value::Direct(function) => (None, Some(function), None),
                Value::ScalarPair([function, environment]) => {
                    (None, Some(function), Some(environment))
                }
                Value::Address(_) => {
                    return Err(self.invalid("native indirect callee is not a function value"));
                }
            },
            mir::Callee::Virtual {
                receiver,
                class,
                slot,
            } => {
                let function = self.virtual_function(receiver, class, slot, builder)?;

                (None, Some(function), None)
            }
            mir::Callee::Dynamic { receiver, slot, .. } => {
                let function = self.dynamic_function(receiver, slot, builder)?;

                (None, Some(function), None)
            }
        };
        if let Some(environment) = environment {
            arguments.push(environment);
        }
        for argument in self.optimized.tree.get_values(call.arguments) {
            self.value(*argument)?.append_values(&mut arguments);
        }

        // issue direct calls without materializing one function address
        let (target, signature) = if let Some(function) = direct {
            let signature = builder.func.dfg.ext_funcs[function].signature;

            (Target::Function(function), signature)
        }
        // resolve function identities through the current native entry table
        else {
            let identity = identity
                .ok_or_else(|| self.invalid("native indirect callee has no function identity"))?;
            let function = self.native_function(identity, frame, builder)?;
            let signature = self
                .types
                .call_signature(call.signature, environment.is_some())?;
            let signature = builder.import_signature(signature);

            (Target::Pointer(function), signature)
        };

        Ok(Call {
            target,
            signature,
            arguments,
            result,
        })
    }

    /// Resolve one callable identity into its current typed native body.
    pub(super) fn native_function(
        &mut self,
        function: cir::Value,
        frame: &FrameMap,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let pointer = self.types.pointer();

        // index the biased table directly with the canonical callable word
        let functions = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, functions),
            builder,
        )?;
        let byte_offset = builder
            .ins()
            .imul_imm_u(function, i64::from(pointer.bytes()));
        let entry = builder.ins().iadd(functions, byte_offset);
        let target = builder
            .ins()
            .load(pointer, cir::MemFlagsData::trusted(), entry, 0);
        let available = builder
            .ins()
            .icmp_imm_u(cir::condcodes::IntCC::NotEqual, target, 0);
        let native = builder.create_block();
        let deoptimize = builder.create_block();
        builder.ins().brif(available, native, &[], deoptimize, &[]);

        // transfer the unchanged call operation to bytecode when no body is installed
        builder.switch_to_block(deoptimize);
        builder.seal_block(deoptimize);
        let id = self.frame_map_id(frame.id, builder)?;
        let call =
            self.emit_runtime(native::abi::Operation::Deopt, &[id, frame.anchor], builder)?;
        Self::attach_stack_map(frame.entries.clone(), call, builder);
        Self::terminate_runtime(builder);

        // continue through the selected typed body on the common path
        builder.switch_to_block(native);
        builder.seal_block(native);

        Ok(target)
    }

    /// Load one virtual method identity from the receiver's linked table.
    fn virtual_function(
        &mut self,
        receiver: mir::Value,
        class: mir::TypeId,
        slot: mir::DispatchSlot,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // read the virtual method layout
        let layout = self
            .optimized
            .layouts
            .type_layout(class)
            .ok_or_else(|| self.invalid("native virtual receiver has no layout"))?;
        let mir::LayoutShape::Object(layout) = &layout.shape else {
            return Err(self.invalid("native virtual receiver is not an object"));
        };
        let offset = layout
            .dispatch_offset
            .ok_or_else(|| self.invalid("native virtual receiver has no dispatch field"))?;
        let receiver = self.materialize_pointer(receiver, builder)?;
        let table = builder.ins().load(
            cir::types::I32,
            cir::MemFlagsData::trusted(),
            receiver,
            offset as i32,
        );

        self.virtual_entry(table, slot.0, builder)
    }

    /// Load one dynamic method identity from the erased value's linked table.
    fn dynamic_function(
        &mut self,
        receiver: mir::Value,
        slot: mir::DispatchSlot,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let (_, function) = self.dynamic_entry(receiver, slot, builder)?;
        let function = builder.ins().uextend(self.types.pointer(), function);

        Ok(builder
            .ins()
            .iadd_imm_u(function, program::FunctionId::WORD_BIAS as i64))
    }

    /// Load one function identity from a process-local virtual table.
    fn virtual_entry(
        &self,
        table: cir::Value,
        slot: u32,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let pointer = self.types.pointer();
        let tables = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, virtuals),
            builder,
        )?;
        let table = builder.ins().uextend(pointer, table);
        let table_offset = builder
            .ins()
            .ishl_imm_u(table, i64::from(pointer.bytes().trailing_zeros()));
        let table_address = builder.ins().iadd(tables, table_offset);
        let table_address =
            builder
                .ins()
                .load(pointer, cir::MemFlagsData::trusted(), table_address, 0);
        let function = builder.ins().load(
            cir::types::I32,
            cir::MemFlagsData::trusted(),
            table_address,
            native::abi::VirtualTable::entry_offset(slot) as i32,
        );
        let function = builder.ins().uextend(pointer, function);

        Ok(builder
            .ins()
            .iadd_imm_u(function, program::FunctionId::WORD_BIAS as i64))
    }

    /// Return whether one call selects an imported binding.
    fn is_imported_binding(&self, call: &mir::Call) -> Result<bool, EmitError> {
        // select direct callees for binding lookup
        let mir::Callee::Direct { function, .. } = call.callee else {
            return Ok(false);
        };
        let function = self.optimized.tree.get(function);

        Ok(!function.is_defined() && function.binding.is_some())
    }
}
