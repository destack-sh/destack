use destack_mir as mir;
use destack_program::Signature;
use destack_program::vm::{
    Call, CallDynamic, CallVirtual, Drop, FunctionBind, IndirectCall, IndirectTailCall,
    Instruction, Invoke, InvokeDynamic, InvokeIndirect, InvokeVirtual, Op, ProjectionId, TailCall,
    TailCallDynamic, TailCallVirtual,
};

use crate::LinkResult;

use super::lower::BlockLowerer;
use super::pool::Pool;
use super::resume::InvokeStates;

impl<'a> BlockLowerer<'a> {
    /// Lower one call instruction.
    pub(super) fn lower_call(
        &self,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        match &call.callee {
            mir::Callee::Direct { function } => self.lower_direct_call(*function, call, pool),
            mir::Callee::Indirect { value } => self.lower_indirect_call(*value, call, pool),
            mir::Callee::Virtual { receiver, slot, .. } => {
                self.lower_virtual_call(*receiver, *slot, call, pool)
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                slot,
            } => self.lower_dynamic_call(*receiver, *constraint, *slot, call, pool),
        }
    }

    /// Lower one concrete value drop.
    pub(super) fn lower_drop(
        &self,
        value: mir::Value,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let ty = self.value_type_for_value(value)?;

        // dynamic carriers release their managed payload root directly
        let repr = self.function.tree.repr_type(ty);
        if matches!(self.function.tree.get(repr), mir::Type::Dynamic { .. }) {
            return self.lower_dynamic_drop(value);
        }

        let Some(function) = self.function.program.program().destructor(ty) else {
            return Err(self.invalid_instruction("destructor"));
        };

        Ok(pool.instruction_with_side(
            Op::Drop,
            Drop {
                function: self.function.program_function(function).0,
                target: self.call_target(function)?,
                value_offset: self.value_offset(value)?,
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
        // resolve the environment cell layout
        let environment_type = self.value_type_for_value(environment)?;
        let environment_layout = self
            .function
            .cell_layout_for_type(environment_type)
            .ok_or_else(|| self.invalid_instruction("function bind environment"))?;

        // pool the environment table payload
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

    /// Lower one direct call.
    fn lower_direct_call(
        &self,
        function: mir::FunctionId,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        // resolve callee and arguments
        let arguments = self.function.values(call.arguments);
        let argument_range = pool.argument_range(arguments)?;

        // compute frame moves once during linking
        let callee = self.function.tree.get(function);
        let moves = pool.parameter_move_range(&callee.parameters, arguments)?;
        let target = self.call_target(function)?;

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

    /// Lower one indirect call.
    fn lower_indirect_call(
        &self,
        value: mir::Value,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let callee = self.indirect_callee(value)?;
        let op = callee.call_op();
        let signature = pool.signature(callee.signature);

        Ok(pool.instruction_with_side(
            op,
            IndirectCall {
                callee_offset: callee.offset,
                signature,
                arguments,
            },
        ))
    }

    /// Lower one virtual call.
    fn lower_virtual_call(
        &self,
        receiver: mir::Value,
        slot: mir::DispatchSlot,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let space = self.receiver_space(receiver)?;
        let table_field = self.virtual_table_field(receiver, pool)?;
        let op = self
            .virtual_call_op(space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{space:?}")))?;

        Ok(pool.instruction_with_side(
            op,
            CallVirtual {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: slot.0,
                arguments,
            },
        ))
    }

    /// Lower one dynamic call.
    fn lower_dynamic_call(
        &self,
        receiver: mir::Value,
        constraint: mir::TypeId,
        slot: mir::DispatchSlot,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let dynamic = self.resolve_dynamic(receiver, constraint)?;

        Ok(pool.instruction_with_side(
            Op::CallDynamic,
            CallDynamic {
                receiver_offset: dynamic.offset,
                slot: slot.0,
                arguments,
            },
        ))
    }

    /// Lower one invocation.
    pub(super) fn lower_invoke(
        &self,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        match &call.callee {
            mir::Callee::Direct { function } => self.lower_direct_invoke(*function, call, pool),
            mir::Callee::Indirect { value } => self.lower_indirect_invoke(*value, call, pool),
            mir::Callee::Virtual { receiver, slot, .. } => {
                self.lower_virtual_invoke(*receiver, *slot, call, pool)
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                slot,
            } => self.lower_dynamic_invoke(*receiver, *constraint, *slot, call, pool),
        }
    }

    /// Lower one direct invocation.
    fn lower_direct_invoke(
        &self,
        function: mir::FunctionId,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let states = self.invoke_states()?;

        Ok(pool.instruction_with_side(
            Op::Invoke,
            Invoke {
                function: self.function.program_function(function).0,
                target: self.call_target(function)?,
                arguments,
                normal_state: states.normal,
                unwind_state: states.unwind,
            },
        ))
    }

    /// Lower one indirect invocation.
    fn lower_indirect_invoke(
        &self,
        value: mir::Value,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let callee = self.indirect_callee(value)?;
        let op = callee.invoke_op();
        let signature = pool.signature(callee.signature);
        let states = self.invoke_states()?;

        Ok(pool.instruction_with_side(
            op,
            InvokeIndirect {
                callee_offset: callee.offset,
                signature,
                arguments,
                normal_state: states.normal,
                unwind_state: states.unwind,
            },
        ))
    }

    /// Lower one virtual invocation.
    fn lower_virtual_invoke(
        &self,
        receiver: mir::Value,
        slot: mir::DispatchSlot,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let states = self.invoke_states()?;
        let space = self.receiver_space(receiver)?;
        let table_field = self.virtual_table_field(receiver, pool)?;
        let op = self
            .virtual_invoke_op(space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{space:?}")))?;

        Ok(pool.instruction_with_side(
            op,
            InvokeVirtual {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: slot.0,
                arguments,
                normal_state: states.normal,
                unwind_state: states.unwind,
            },
        ))
    }

    /// Lower one dynamic invocation.
    fn lower_dynamic_invoke(
        &self,
        receiver: mir::Value,
        constraint: mir::TypeId,
        slot: mir::DispatchSlot,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let states = self.invoke_states()?;
        let dynamic = self.resolve_dynamic(receiver, constraint)?;

        Ok(pool.instruction_with_side(
            Op::InvokeDynamic,
            InvokeDynamic {
                receiver_offset: dynamic.offset,
                slot: slot.0,
                arguments,
                normal_state: states.normal,
                unwind_state: states.unwind,
            },
        ))
    }

    /// Lower one tail call.
    pub(super) fn lower_tail_call(
        &self,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        match &call.callee {
            mir::Callee::Direct { function } => self.lower_direct_tail_call(*function, call, pool),
            mir::Callee::Indirect { value } => self.lower_indirect_tail_call(*value, call, pool),
            mir::Callee::Virtual { receiver, slot, .. } => {
                self.lower_virtual_tail_call(*receiver, *slot, call, pool)
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                slot,
            } => self.lower_dynamic_tail_call(*receiver, *constraint, *slot, call, pool),
        }
    }

    /// Lower one direct tail call.
    fn lower_direct_tail_call(
        &self,
        function: mir::FunctionId,
        call: &mir::Call,
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
        value: mir::Value,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let callee = self.indirect_callee(value)?;
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
        slot: mir::DispatchSlot,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let space = self.receiver_space(receiver)?;
        let table_field = self.virtual_table_field(receiver, pool)?;
        let op = self
            .virtual_tail_call_op(space)
            .ok_or_else(|| self.invalid_pointer_type(format!("{space:?}")))?;

        Ok(pool.instruction_with_side(
            op,
            TailCallVirtual {
                receiver_offset: self.cell_offset(receiver)?,
                table_field,
                slot: slot.0,
                arguments,
            },
        ))
    }

    /// Lower one dynamic tail call.
    fn lower_dynamic_tail_call(
        &self,
        receiver: mir::Value,
        constraint: mir::TypeId,
        slot: mir::DispatchSlot,
        call: &mir::Call,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        let arguments = pool.argument_range(self.function.values(call.arguments))?;
        let dynamic = self.resolve_dynamic(receiver, constraint)?;

        Ok(pool.instruction_with_side(
            Op::TailCallDynamic,
            TailCallDynamic {
                receiver_offset: dynamic.offset,
                slot: slot.0,
                arguments,
            },
        ))
    }

    /// Return the frame states for the current invocation.
    fn invoke_states(&self) -> LinkResult<InvokeStates> {
        self.function
            .invoke_frame_states
            .get(&self.block_id())
            .copied()
            .ok_or_else(|| {
                self.internal(format!(
                    "missing invocation frame states: {:?}",
                    self.block_id()
                ))
            })
    }

    /// Return the static callee representation for one indirect call.
    fn indirect_callee(&self, value: mir::Value) -> LinkResult<IndirectCallee> {
        let callee_type = self.value_type_for_value(value)?;
        let callee_type = self.function.tree.repr_type(callee_type);

        // distinguish bare function pointers from closures
        let (signature, has_environment) = match self.function.tree.get(callee_type) {
            mir::Type::FunctionPointer { signature } => (*signature, false),
            mir::Type::Function { signature, .. } => (*signature, true),
            _ => return Err(self.invalid_instruction("indirect callee")),
        };
        let offset = if has_environment {
            self.value_offset(value)?
        } else {
            self.cell_offset(value)?
        };
        let signature = self
            .function
            .type_linker()
            .signature(signature)
            .ok_or_else(|| self.invalid_instruction("indirect callee signature"))?;

        Ok(IndirectCallee {
            offset,
            signature,
            has_environment,
        })
    }

    /// Pool the virtual dispatch table field for one receiver.
    fn virtual_table_field(
        &self,
        receiver: mir::Value,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<ProjectionId> {
        let receiver_type = self.receiver_pointee_type(receiver);
        let projection = self
            .virtual_table_projection(receiver_type)
            .ok_or_else(|| self.invalid_instruction("virtual call table field"))?;

        Ok(pool.projection(projection))
    }

    /// Return the heap pointee type for one virtual receiver.
    fn receiver_pointee_type(&self, receiver: mir::Value) -> Option<mir::TypeId> {
        self.operand_map()
            .heap_pointee_type(self.function.program, receiver)
    }

    /// Return the storage space for one virtual receiver.
    fn receiver_space(&self, receiver: mir::Value) -> LinkResult<mir::Space> {
        self.operand_map()
            .space(receiver)
            .ok_or_else(|| self.invalid_pointer_type(format!("{receiver:?}")))
    }

    /// Select one virtual call opcode.
    fn virtual_call_op(&self, space: mir::Space) -> Option<Op> {
        match space {
            mir::Space::Local => Some(Op::CallVirtualLocal),
            mir::Space::Shared => Some(Op::CallVirtualShared),
            _ => None,
        }
    }

    /// Select one virtual invocation opcode.
    fn virtual_invoke_op(&self, space: mir::Space) -> Option<Op> {
        match space {
            mir::Space::Local => Some(Op::InvokeVirtualLocal),
            mir::Space::Shared => Some(Op::InvokeVirtualShared),
            _ => None,
        }
    }

    /// Select one virtual tail call opcode.
    fn virtual_tail_call_op(&self, space: mir::Space) -> Option<Op> {
        match space {
            mir::Space::Local => Some(Op::TailCallVirtualLocal),
            mir::Space::Shared => Some(Op::TailCallVirtualShared),
            _ => None,
        }
    }
}

/// One lowered indirect callee.
struct IndirectCallee {
    /// The callee offset in the current frame.
    offset: u32,
    /// The expected function signature.
    signature: Signature,
    /// Whether the callee carries an environment reference.
    has_environment: bool,
}

impl IndirectCallee {
    /// Select one indirect call opcode.
    fn call_op(&self) -> Op {
        if self.has_environment {
            Op::CallFunction
        } else {
            Op::CallFunctionPointer
        }
    }

    /// Select one indirect invocation opcode.
    fn invoke_op(&self) -> Op {
        if self.has_environment {
            Op::InvokeFunction
        } else {
            Op::InvokeFunctionPointer
        }
    }

    /// Select one indirect tail call opcode.
    fn tail_call_op(&self) -> Op {
        if self.has_environment {
            Op::TailCallFunction
        } else {
            Op::TailCallFunctionPointer
        }
    }
}
