use destack_mir as mir;

use crate::LinkResult;
use destack_program::vm::{
    Call, CallBranch, CallDynamic, CallDynamicBranch, CallVirtual, CallVirtualBranch, FunctionBind,
    IndirectCall, IndirectCallBranch, IndirectTailCall, Instruction, Op, TailCall, TailCallDynamic,
    TailCallVirtual,
};
use destack_program::{AddressSpace, FrameStateId, Signature};

use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one call-like terminator.
    pub(super) fn lower_call_terminator(
        &self,
        term: &mir::Terminator,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        Ok(match term {
            mir::Terminator::Call { function, call, .. } => {
                self.lower_call_branch(*function, call, pool)?
            }
            mir::Terminator::CallIndirect { callee, call, .. } => {
                self.lower_indirect_call_branch(*callee, call, pool)?
            }
            mir::Terminator::CallVirtual {
                receiver,
                slot,
                call,
                ..
            } => self.lower_virtual_call_branch(*receiver, *slot, call, pool)?,
            mir::Terminator::CallDynamic {
                receiver,
                slot,
                call,
                ..
            } => self.lower_dynamic_call_branch(*receiver, *slot, call, pool)?,
            mir::Terminator::TailCall { function, call, .. } => {
                self.lower_tail_call(*function, call, pool)?
            }
            mir::Terminator::TailCallIndirect { callee, call, .. } => {
                self.lower_indirect_tail_call(*callee, call, pool)?
            }
            mir::Terminator::TailCallVirtual {
                receiver,
                slot,
                call,
                ..
            } => self.lower_virtual_tail_call(*receiver, *slot, call, pool)?,
            mir::Terminator::TailCallDynamic {
                receiver,
                slot,
                call,
                ..
            } => self.lower_dynamic_tail_call(*receiver, *slot, call, pool)?,
            _ => return Err(self.invalid_instruction("call terminator")),
        })
    }

    /// Lower one direct call.
    pub(super) fn lower_call(
        &self,
        _destination: Option<mir::Value>,
        function: mir::FunctionId,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        // resolve callee and arguments
        let arguments = self.function.tree.get_values(call.arguments);
        let argument_range = pool.argument_range(arguments)?;

        // compute frame moves once during lowering
        let callee = self.function.tree.get(function);
        let moves = pool.parameter_move_range(&callee.parameters, arguments)?;
        let target = self.call_target(function)?;

        // emit the compact call instruction
        Ok(pool.instruction_with_side(
            Op::Call,
            Call {
                function: self.function.program_function(function).0,
                target,
                arguments: argument_range,
                moves,
            },
        ))
    }

    /// Lower one virtual call.
    pub(super) fn lower_virtual_call(
        &self,
        _destination: Option<mir::Value>,
        receiver: mir::Value,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        // resolve arguments and receiver
        let arguments = pool.argument_range(self.function.tree.get_values(call.arguments))?;

        // compile the receiver table access
        let address_space = self.receiver_address_space(receiver)?;
        let receiver_type = self.receiver_pointee_type(receiver);
        let table_field = self
            .virtual_table_projection(receiver_type)
            .ok_or_else(|| self.invalid_instruction("virtual call table field"))?;
        let table_field = pool.projection(table_field);
        let op = virtual_call_op(address_space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{address_space:?}")))?;

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            op,
            CallVirtual {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Lower one dynamic call.
    pub(super) fn lower_dynamic_call(
        &self,
        _destination: Option<mir::Value>,
        receiver: mir::Value,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        // resolve arguments and receiver
        let arguments = pool.argument_range(self.function.tree.get_values(call.arguments))?;

        // compile the receiver table access
        let address_space = self.receiver_address_space(receiver)?;
        let receiver_type = self.receiver_pointee_type(receiver);
        let table_field = self
            .dynamic_table_projection(receiver_type)
            .ok_or_else(|| self.invalid_instruction("dynamic call table field"))?;
        let table_field = pool.projection(table_field);
        let op = dynamic_call_op(address_space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{address_space:?}")))?;

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            op,
            CallDynamic {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Lower one indirect call.
    pub(super) fn lower_indirect_call(
        &self,
        _destination: Option<mir::Value>,
        callee: mir::Value,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        // resolve call arguments
        let arguments = pool.argument_range(self.function.tree.get_values(call.arguments))?;

        // resolve function callee
        let callee = self.indirect_callee(callee)?;
        let op = callee.call_op();
        let signature = pool.signature(callee.signature);

        // emit the function pointer or function opcode
        Ok(pool.instruction_with_side(
            op,
            IndirectCall {
                callee_offset: callee.offset,
                signature,
                arguments,
            },
        ))
    }

    /// Lower one function bind.
    pub(super) fn lower_function_bind(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        function: mir::FunctionId,
        environment: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve the environment word layout
        let environment_type = self.value_type_for_value(environment)?;
        let environment_layout = self
            .function
            .cell_layout_for_type(environment_type)
            .ok_or_else(|| self.invalid_instruction("function bind environment"))?;

        // pool the cold environment tables
        let bind = FunctionBind {
            environment: environment_layout,
        };
        let bind = pool.side_record(bind);

        // put the hot operands in the instruction payload
        Ok(Instruction::new(
            Op::FunctionBind,
            self.value_offset(destination)?,
            self.function.program_function(function).0,
            self.cell_offset(environment)?,
            bind,
        ))
    }

    /// Lower one function pointer projection.
    pub(super) fn lower_function_pointer(
        &self,
        destination: mir::Value,
        function: mir::Value,
    ) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::FunctionPointer,
            self.cell_offset(destination)?,
            self.value_offset(function)?,
            0,
            0,
        ))
    }

    /// Lower one function environment projection.
    pub(super) fn lower_function_environment(
        &self,
        destination: mir::Value,
        function: mir::Value,
    ) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::FunctionEnvironment,
            self.cell_offset(destination)?,
            self.value_offset(function)?,
            0,
            0,
        ))
    }

    /// Lower one current function environment read.
    pub(super) fn lower_function_environment_current(
        &self,
        destination: mir::Value,
    ) -> LinkResult<Instruction> {
        Ok(Instruction::new(
            Op::FunctionEnvironmentCurrent,
            self.cell_offset(destination)?,
            0,
            0,
            0,
        ))
    }

    /// Return the frame state for a call terminator.
    fn call_target_state(&self) -> LinkResult<FrameStateId> {
        self.function
            .call_frame_states
            .get(&self.block_id())
            .copied()
            .ok_or_else(|| {
                self.internal(format!("missing call frame state: {:?}", self.block_id()))
            })
    }

    /// Return the static callee for one indirect call.
    fn indirect_callee(&self, callee: mir::Value) -> LinkResult<IndirectCallee> {
        let callee_type = self.value_type_for_value(callee)?;
        let callee_type = self.function.tree.repr_type(callee_type);

        // function pointers carry only the target function id
        let (signature, has_environment) = match self.function.tree.get(callee_type) {
            mir::Type::FunctionPointer { signature } => (*signature, false),
            mir::Type::Function { signature, .. } => (*signature, true),
            _ => return Err(self.invalid_instruction("indirect callee")),
        };

        let offset = if has_environment {
            self.value_offset(callee)?
        } else {
            self.cell_offset(callee)?
        };

        Ok(IndirectCallee {
            offset,
            signature: self
                .function
                .type_linker()
                .signature(signature)
                .ok_or_else(|| self.invalid_instruction("indirect callee signature"))?,
            has_environment,
        })
    }

    /// Lower one direct call terminator.
    fn lower_call_branch(
        &self,
        function: mir::FunctionId,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        let arguments = pool.argument_range(arguments)?;
        let target_state = self.call_target_state()?;

        Ok(pool.instruction_with_side(
            Op::CallBranch,
            CallBranch {
                function: self.function.program_function(function).0,
                target: self.call_target(function)?,
                arguments,
                target_state,
            },
        ))
    }

    /// Lower one indirect call terminator.
    fn lower_indirect_call_branch(
        &self,
        callee: mir::Value,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        let arguments = pool.argument_range(arguments)?;
        let target_state = self.call_target_state()?;
        let callee = self.indirect_callee(callee)?;
        let op = callee.call_branch_op();
        let signature = pool.signature(callee.signature);

        Ok(pool.instruction_with_side(
            op,
            IndirectCallBranch {
                callee_offset: callee.offset,
                signature,
                arguments,
                target_state,
            },
        ))
    }

    /// Lower one virtual call terminator.
    fn lower_virtual_call_branch(
        &self,
        receiver: mir::Value,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        let arguments = pool.argument_range(arguments)?;
        let target_state = self.call_target_state()?;
        let address_space = self.receiver_address_space(receiver)?;
        let receiver_type = self.receiver_pointee_type(receiver);
        let table_field = self
            .virtual_table_projection(receiver_type)
            .ok_or_else(|| self.invalid_instruction("virtual call table field"))?;
        let table_field = pool.projection(table_field);
        let op = virtual_call_branch_op(address_space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{address_space:?}")))?;

        Ok(pool.instruction_with_side(
            op,
            CallVirtualBranch {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: method.0,
                arguments,
                target_state,
            },
        ))
    }

    /// Lower one dynamic call terminator.
    fn lower_dynamic_call_branch(
        &self,
        receiver: mir::Value,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        let arguments = pool.argument_range(arguments)?;
        let target_state = self.call_target_state()?;
        let address_space = self.receiver_address_space(receiver)?;
        let receiver_type = self.receiver_pointee_type(receiver);
        let table_field = self
            .dynamic_table_projection(receiver_type)
            .ok_or_else(|| self.invalid_instruction("dynamic call table field"))?;
        let table_field = pool.projection(table_field);
        let op = dynamic_call_branch_op(address_space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{address_space:?}")))?;

        Ok(pool.instruction_with_side(
            op,
            CallDynamicBranch {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: method.0,
                arguments,
                target_state,
            },
        ))
    }

    /// Lower one direct tail call.
    fn lower_tail_call(
        &self,
        function: mir::FunctionId,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        if function == self.function.function_id {
            let arguments = pool.argument_range(arguments)?;

            return Ok(Instruction::new(
                Op::TailCallSelf,
                self.function.entry_block,
                arguments.start,
                arguments.len,
                0,
            ));
        }

        let callee = self.function.tree.get(function);
        let moves = pool.parameter_move_range(&callee.parameters, arguments)?;
        let target = self.call_target(function)?;

        Ok(pool.instruction_with_side(
            Op::TailCall,
            TailCall {
                function: self.function.program_function(function).0,
                target,
                moves,
            },
        ))
    }

    /// Lower one indirect tail call.
    fn lower_indirect_tail_call(
        &self,
        callee: mir::Value,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        let arguments = pool.argument_range(arguments)?;
        let callee = self.indirect_callee(callee)?;
        let op = callee.tail_call_op();
        let signature = pool.signature(callee.signature);

        Ok(pool.instruction_with_side(
            op,
            IndirectTailCall {
                callee_offset: callee.offset,
                signature,
                arguments,
            },
        ))
    }

    /// Lower one virtual tail call.
    fn lower_virtual_tail_call(
        &self,
        receiver: mir::Value,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        let arguments = pool.argument_range(arguments)?;
        let address_space = self.receiver_address_space(receiver)?;
        let receiver_type = self.receiver_pointee_type(receiver);
        let table_field = self
            .virtual_table_projection(receiver_type)
            .ok_or_else(|| self.invalid_instruction("virtual tail call table field"))?;
        let table_field = pool.projection(table_field);
        let op = virtual_tail_call_op(address_space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{address_space:?}")))?;

        Ok(pool.instruction_with_side(
            op,
            TailCallVirtual {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Lower one dynamic tail call.
    fn lower_dynamic_tail_call(
        &self,
        receiver: mir::Value,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = self.function.values(call.arguments);
        let arguments = pool.argument_range(arguments)?;
        let address_space = self.receiver_address_space(receiver)?;
        let receiver_type = self.receiver_pointee_type(receiver);
        let table_field = self
            .dynamic_table_projection(receiver_type)
            .ok_or_else(|| self.invalid_instruction("dynamic tail call table field"))?;
        let table_field = pool.projection(table_field);
        let op = dynamic_tail_call_op(address_space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{address_space:?}")))?;

        Ok(pool.instruction_with_side(
            op,
            TailCallDynamic {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Return the heap pointee type for one virtual or dynamic receiver.
    fn receiver_pointee_type(&self, receiver: mir::Value) -> Option<mir::LocalNodeId<mir::Type>> {
        self.operand_map()
            .heap_pointee_type(self.function.program, receiver)
    }

    /// Return the address space for one virtual or dynamic receiver.
    fn receiver_address_space(&self, receiver: mir::Value) -> LinkResult<AddressSpace> {
        self.operand_map()
            .address_space(receiver)
            .ok_or_else(|| self.invalid_pointer_type(format!("{receiver:?}")))
    }
}

/// Static callee for one indirect call.
struct IndirectCallee {
    /// Cell offset of the function value in the current frame.
    offset: u32,
    /// Expected function signature.
    signature: Signature,
    /// Whether the function value carries an environment pointer.
    has_environment: bool,
}

impl IndirectCallee {
    /// Select one indirect call opcode.
    fn call_op(&self) -> Op {
        if self.has_environment {
            return Op::CallFunction;
        }

        Op::CallFunctionPointer
    }

    /// Select one indirect call terminator opcode.
    fn call_branch_op(&self) -> Op {
        if self.has_environment {
            return Op::CallFunctionBranch;
        }

        Op::CallFunctionPointerBranch
    }

    /// Select one indirect tail call opcode.
    fn tail_call_op(&self) -> Op {
        if self.has_environment {
            return Op::TailCallFunction;
        }

        Op::TailCallFunctionPointer
    }
}

/// Return the virtual call op for one receiver address space.
fn virtual_call_op(address_space: AddressSpace) -> Option<Op> {
    match address_space {
        AddressSpace::Local => Some(Op::CallVirtualLocal),
        AddressSpace::Shared => Some(Op::CallVirtualShared),
        _ => None,
    }
}

/// Return the virtual call terminator op for one receiver address space.
fn virtual_call_branch_op(address_space: AddressSpace) -> Option<Op> {
    match address_space {
        AddressSpace::Local => Some(Op::CallVirtualLocalBranch),
        AddressSpace::Shared => Some(Op::CallVirtualSharedBranch),
        _ => None,
    }
}

/// Return the virtual tail call op for one receiver address space.
fn virtual_tail_call_op(address_space: AddressSpace) -> Option<Op> {
    match address_space {
        AddressSpace::Local => Some(Op::TailCallVirtualLocal),
        AddressSpace::Shared => Some(Op::TailCallVirtualShared),
        _ => None,
    }
}

/// Return the dynamic call op for one receiver address space.
fn dynamic_call_op(address_space: AddressSpace) -> Option<Op> {
    match address_space {
        AddressSpace::Local => Some(Op::CallDynamicLocal),
        AddressSpace::Shared => Some(Op::CallDynamicShared),
        _ => None,
    }
}

/// Return the dynamic call terminator op for one receiver address space.
fn dynamic_call_branch_op(address_space: AddressSpace) -> Option<Op> {
    match address_space {
        AddressSpace::Local => Some(Op::CallDynamicLocalBranch),
        AddressSpace::Shared => Some(Op::CallDynamicSharedBranch),
        _ => None,
    }
}

/// Return the dynamic tail call op for one receiver address space.
fn dynamic_tail_call_op(address_space: AddressSpace) -> Option<Op> {
    match address_space {
        AddressSpace::Local => Some(Op::TailCallDynamicLocal),
        AddressSpace::Shared => Some(Op::TailCallDynamicShared),
        _ => None,
    }
}
