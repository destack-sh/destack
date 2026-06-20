use crate::{Error, Result};
use destack_mir as mir;
use destack_program::vm::{
    AddressSpace, Call, CallBranch, CallDynamic, CallDynamicBranch, CallVirtual, CallVirtualBranch,
    FunctionBind, IndirectCall, IndirectCallBranch, IndirectTailCall, Instruction, Op, TailCall,
    TailCallDynamic, TailCallVirtual, cell_layout_from_type,
};

use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::projection::{dynamic_table_projection, virtual_table_projection};
use super::value::{
    address_space_for_value, heap_pointee_type_for_value, heap_pointee_type_from_shape,
};

impl<'a> BlockLowerer<'a> {
    /// Lower one call-like terminator.
    pub(super) fn lower_call_terminator(
        &self,
        term: &mir::Terminator,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
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
            _ => return Err(Error::invalid_instruction()),
        })
    }

    /// Lower one direct call.
    pub(super) fn lower_call(
        &self,
        _destination: Option<mir::Value>,
        function: mir::FunctionId,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve callee and arguments
        let arguments = self.tree.get_values(call.arguments);
        let argument_range = pool.argument_range(arguments);

        // compute frame moves once during lowering
        let callee = self.tree.get(function);
        let moves = pool.parameter_move_range(&callee.parameters, arguments)?;
        let target = self.call_target(function)?;

        // emit the compact call instruction
        Ok(pool.instruction_with_side(
            Op::Call,
            Call {
                function: self.program_function(function).0,
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
    ) -> Result<Instruction> {
        // resolve arguments and receiver
        let arguments = pool.argument_range(self.tree.get_values(call.arguments));

        // compile the receiver table access
        let address_space = address_space_for_value(self.value_shape_map(), receiver)?;
        let table_field = virtual_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("virtual call table field"))?;
        let table_field = pool.projection(table_field);

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            virtual_call_op(address_space)?,
            CallVirtual {
                receiver_offset: cell_offset(self, receiver)?,
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
    ) -> Result<Instruction> {
        // resolve arguments and receiver
        let arguments = pool.argument_range(self.tree.get_values(call.arguments));

        // compile the receiver table access
        let address_space = address_space_for_value(self.value_shape_map(), receiver)?;
        let table_field = dynamic_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("dynamic call table field"))?;
        let table_field = pool.projection(table_field);

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            dynamic_call_op(address_space)?,
            CallDynamic {
                receiver_offset: cell_offset(self, receiver)?,
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
    ) -> Result<Instruction> {
        // resolve call arguments
        let arguments = pool.argument_range(self.tree.get_values(call.arguments));

        // resolve function shape
        let callee = self.indirect_callee(callee)?;

        // emit the function pointer or function opcode
        Ok(pool.instruction_with_side(
            indirect_call_op(&callee),
            IndirectCall {
                callee_offset: callee.offset,
                signature: callee.signature,
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
    ) -> Result<Instruction> {
        // resolve the environment word layout
        let environment_type = self.value_type_for_value(environment)?;
        let environment_layout = cell_layout_from_type(self.tree, environment_type)
            .ok_or(Error::invalid_instruction())?;

        // pool the cold environment metadata
        let bind = FunctionBind {
            environment: environment_layout,
        };
        let bind = pool.side_record(bind);

        // put the hot operands in the instruction payload
        Ok(Instruction::new(
            Op::FunctionBind,
            value_offset(self, destination)?,
            self.program_function(function).0,
            cell_offset(self, environment)?,
            bind,
        ))
    }

    /// Lower one function pointer projection.
    pub(super) fn lower_function_pointer(
        &self,
        destination: mir::Value,
        function: mir::Value,
    ) -> Result<Instruction> {
        Ok(Instruction::new(
            Op::FunctionPointer,
            cell_offset(self, destination)?,
            value_offset(self, function)?,
            0,
            0,
        ))
    }

    /// Lower one function environment projection.
    pub(super) fn lower_function_environment(
        &self,
        destination: mir::Value,
        function: mir::Value,
    ) -> Result<Instruction> {
        Ok(Instruction::new(
            Op::FunctionEnvironment,
            cell_offset(self, destination)?,
            value_offset(self, function)?,
            0,
            0,
        ))
    }

    /// Lower one current function environment read.
    pub(super) fn lower_function_environment_current(
        &self,
        destination: mir::Value,
    ) -> Result<Instruction> {
        Ok(Instruction::new(
            Op::FunctionEnvironmentCurrent,
            cell_offset(self, destination)?,
            0,
            0,
            0,
        ))
    }

    /// Return the frame state for a call terminator.
    fn call_target_state(&self) -> Result<mir::FrameStateId> {
        self.call_frame_states
            .get(&self.block_id())
            .copied()
            .ok_or_else(|| {
                Error::internal(format!(
                    "missing call frame state for block: {:?}",
                    self.block_id()
                ))
            })
    }

    /// Return static callee shape for one indirect call.
    fn indirect_callee(&self, callee: mir::Value) -> Result<IndirectCallee> {
        let callee_type = self.value_type_for_value(callee)?;
        let callee_type = self.tree.repr_type(callee_type);

        // function pointers carry only the target function id
        let (signature, has_environment) = match self.tree.get(callee_type) {
            mir::Type::FunctionPointer { signature } => (*signature, false),
            mir::Type::Function { signature, .. } => (*signature, true),
            _ => return Err(Error::invalid_instruction()),
        };

        let offset = if has_environment {
            value_offset(self, callee)?
        } else {
            cell_offset(self, callee)?
        };

        Ok(IndirectCallee {
            offset,
            signature,
            has_environment,
        })
    }

    /// Lower one direct call terminator.
    fn lower_call_branch(
        &self,
        function: mir::FunctionId,
        call: &mir::Call<mir::ValueSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        let arguments = pool.argument_range(arguments);
        let target_state = self.call_target_state()?;

        Ok(pool.instruction_with_side(
            Op::CallBranch,
            CallBranch {
                function: self.program_function(function).0,
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
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        let arguments = pool.argument_range(arguments);
        let target_state = self.call_target_state()?;
        let callee = self.indirect_callee(callee)?;

        Ok(pool.instruction_with_side(
            indirect_call_branch_op(&callee),
            IndirectCallBranch {
                callee_offset: callee.offset,
                signature: callee.signature,
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
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        let arguments = pool.argument_range(arguments);
        let target_state = self.call_target_state()?;
        let address_space = address_space_for_value(self.value_shape_map(), receiver)?;
        let table_field = virtual_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("call virtual table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            virtual_call_branch_op(address_space)?,
            CallVirtualBranch {
                receiver_offset: cell_offset(self, receiver)?,
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
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        let arguments = pool.argument_range(arguments);
        let target_state = self.call_target_state()?;
        let address_space = address_space_for_value(self.value_shape_map(), receiver)?;
        let table_field = dynamic_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("call dynamic table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            dynamic_call_branch_op(address_space)?,
            CallDynamicBranch {
                receiver_offset: cell_offset(self, receiver)?,
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
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        if function == self.function_id {
            let arguments = pool.argument_range(arguments);

            return Ok(Instruction::new(
                Op::TailCallSelf,
                self.entry_block,
                arguments.start,
                arguments.len,
                0,
            ));
        }

        let callee = self.tree.get(function);
        let moves = pool.parameter_move_range(&callee.parameters, arguments)?;
        let target = self.call_target(function)?;

        Ok(pool.instruction_with_side(
            Op::TailCall,
            TailCall {
                function: self.program_function(function).0,
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
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        let arguments = pool.argument_range(arguments);
        let callee = self.indirect_callee(callee)?;

        Ok(pool.instruction_with_side(
            indirect_tail_call_op(&callee),
            IndirectTailCall {
                callee_offset: callee.offset,
                signature: callee.signature,
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
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        let arguments = pool.argument_range(arguments);
        let address_space = address_space_for_value(self.value_shape_map(), receiver)?;
        let table_field = virtual_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_from_shape(self.types, self.value_shape_map(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("tail virtual table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            virtual_tail_call_op(address_space)?,
            TailCallVirtual {
                receiver_offset: cell_offset(self, receiver)?,
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
    ) -> Result<Instruction> {
        let arguments = self.values(call.arguments);
        let arguments = pool.argument_range(arguments);
        let address_space = address_space_for_value(self.value_shape_map(), receiver)?;
        let table_field = dynamic_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_from_shape(self.types, self.value_shape_map(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("tail dynamic table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            dynamic_tail_call_op(address_space)?,
            TailCallDynamic {
                receiver_offset: cell_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }
}

/// Static callee shape for one indirect call.
struct IndirectCallee {
    /// Cell offset of the function value in the current frame.
    offset: u32,
    /// Expected function signature.
    signature: mir::LocalNodeId<mir::Type>,
    /// Whether the function value carries an environment pointer.
    has_environment: bool,
}

/// Return the virtual call op for one receiver address space.
fn virtual_call_op(address_space: AddressSpace) -> Result<Op> {
    match address_space {
        AddressSpace::Local => Ok(Op::CallVirtualLocal),
        AddressSpace::Shared => Ok(Op::CallVirtualShared),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Return the virtual call terminator op for one receiver address space.
fn virtual_call_branch_op(address_space: AddressSpace) -> Result<Op> {
    match address_space {
        AddressSpace::Local => Ok(Op::CallVirtualLocalBranch),
        AddressSpace::Shared => Ok(Op::CallVirtualSharedBranch),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Return the virtual tail call op for one receiver address space.
fn virtual_tail_call_op(address_space: AddressSpace) -> Result<Op> {
    match address_space {
        AddressSpace::Local => Ok(Op::TailCallVirtualLocal),
        AddressSpace::Shared => Ok(Op::TailCallVirtualShared),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Return the dynamic call op for one receiver address space.
fn dynamic_call_op(address_space: AddressSpace) -> Result<Op> {
    match address_space {
        AddressSpace::Local => Ok(Op::CallDynamicLocal),
        AddressSpace::Shared => Ok(Op::CallDynamicShared),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Return the dynamic call terminator op for one receiver address space.
fn dynamic_call_branch_op(address_space: AddressSpace) -> Result<Op> {
    match address_space {
        AddressSpace::Local => Ok(Op::CallDynamicLocalBranch),
        AddressSpace::Shared => Ok(Op::CallDynamicSharedBranch),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Return the dynamic tail call op for one receiver address space.
fn dynamic_tail_call_op(address_space: AddressSpace) -> Result<Op> {
    match address_space {
        AddressSpace::Local => Ok(Op::TailCallDynamicLocal),
        AddressSpace::Shared => Ok(Op::TailCallDynamicShared),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Select one indirect call opcode from callee shape.
fn indirect_call_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::CallFunction;
    }

    Op::CallFunctionPointer
}

/// Select one indirect call terminator opcode from callee shape.
fn indirect_call_branch_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::CallFunctionBranch;
    }

    Op::CallFunctionPointerBranch
}

/// Select one indirect tail call opcode from callee shape.
fn indirect_tail_call_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::TailCallFunction;
    }

    Op::TailCallFunctionPointer
}
