use destack_engine as engine;
use destack_mir as mir;

use crate::program::{
    Call, CallBranch, CallDynamic, CallDynamicBranch, CallIndirect, CallIndirectBranch,
    CallVirtual, CallVirtualBranch, ClosureBind, ClosureEnvironment, Instruction, Op, PointerClass,
    TailCall, TailCallDynamic, TailCallIndirect, TailCallVirtual, closure_object_layout, repr_type,
    word_layout_from_type,
};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::projection::{dynamic_table_projection, virtual_table_projection};
use super::value::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_layout, pointer_class_for_value,
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
        _destination: Option<mir::ValueReference>,
        function: mir::FunctionReference,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve callee and arguments
        let function = function
            .function()
            .ok_or_else(|| Error::invalid_program("call callee"))?;
        let arguments = self.tree.get_arguments(call.arguments);
        let argument_range = pool.argument_reference_range(arguments, "call argument")?;

        // compute frame moves once during lowering
        let argument_value = arguments
            .iter()
            .map(|argument| {
                argument
                    .value()
                    .ok_or_else(|| Error::invalid_program("call argument"))
            })
            .collect::<Result<Vec<_>>>()?;
        let callee = self.tree.get(function);
        let moves = pool.parameter_move_range(&callee.parameters, &argument_value)?;
        let target = self.call_target(function)?;

        // emit the compact call instruction
        Ok(pool.instruction_with_side(
            Op::Call,
            Call {
                function: function.id,
                target,
                arguments: argument_range,
                moves,
            },
        ))
    }

    /// Lower one virtual call.
    pub(super) fn lower_virtual_call(
        &self,
        _destination: Option<mir::ValueReference>,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve arguments and receiver
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "virtual call argument",
        )?;
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::invalid_program("virtual call receiver"))?;

        // compile the receiver table access
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = virtual_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("virtual call table field"))?;
        let table_field = pool.projection(table_field);

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            virtual_call_op(pointer_class)?,
            CallVirtual {
                receiver_offset: word_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Lower one dynamic call.
    pub(super) fn lower_dynamic_call(
        &self,
        _destination: Option<mir::ValueReference>,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve arguments and receiver
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "dynamic call argument",
        )?;
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::invalid_program("dynamic call receiver"))?;

        // compile the receiver table access
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = dynamic_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("dynamic call table field"))?;
        let table_field = pool.projection(table_field);

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            dynamic_call_op(pointer_class)?,
            CallDynamic {
                receiver_offset: word_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Lower one indirect call.
    pub(super) fn lower_indirect_call(
        &self,
        _destination: Option<mir::ValueReference>,
        callee: mir::ValueReference,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve call arguments
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "indirect call argument",
        )?;

        // resolve closure shape
        let callee = callee
            .value()
            .ok_or_else(|| Error::invalid_program("indirect call callee"))?;
        let callee = self.indirect_callee(callee)?;

        // emit the function pointer or closure opcode
        Ok(pool.instruction_with_side(
            indirect_call_op(&callee),
            CallIndirect {
                callee_offset: callee.offset,
                signature: callee.signature,
                arguments,
            },
        ))
    }

    /// Lower one closure bind.
    pub(super) fn lower_closure_bind(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        function: mir::FunctionReference,
        environment: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve closure operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("closure.bind destination"))?;
        let function = function
            .function()
            .ok_or_else(|| Error::invalid_program("closure.bind callee"))?;
        let environment = environment
            .value()
            .ok_or_else(|| Error::invalid_program("closure.bind environment"))?;

        // select the environment representation
        let destination_type = self.value_type_for_value(destination)?;
        let environment_type = self.value_type_for_value(environment)?;
        let environment_layout = self.layout_for_type(environment_type)?;

        let (op, environment_repr) = if environment_layout.is_word() {
            let layout = word_layout_from_type(self.tree, environment_type)
                .ok_or(Error::invalid_instruction())?;

            (Op::BindClosureWord, ClosureEnvironment::Word { layout })
        } else {
            (
                Op::BindClosureAddress,
                ClosureEnvironment::Frame {
                    layout: self.layout_id_for_type(environment_type)?,
                    byte_len: environment_layout.byte_len,
                },
            )
        };

        // pool the cold closure layout metadata
        let closure = ClosureBind {
            closure_layout: self.layout_id_for_type(destination_type)?,
            object_layout: closure_object_layout(self.tree.pointer_bytes() as usize),
            environment: environment_repr,
        };
        let closure = pool.side_record(closure);

        // put the hot operands in the instruction payload
        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            function.id,
            value_offset(self, environment)?,
            closure,
        ))
    }

    /// Lower one closure environment read.
    pub(super) fn lower_closure_environment(
        &self,
        destination: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("closure environment destination"))?;

        Ok(Instruction::new(
            Op::LoadClosureEnvironment,
            word_offset(self, destination)?,
            0,
            0,
            0,
        ))
    }

    /// Return the frame state for a call terminator.
    fn call_target_state(&self) -> Result<engine::FrameStateId> {
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
        let callee_type = repr_type(self.tree, callee_type);

        // function pointers carry only the target function id
        let (signature, has_environment) = match self.tree.get(callee_type) {
            mir::Type::FunctionPointer { signature } => {
                let signature = signature.ty().ok_or(Error::invalid_instruction())?;

                (signature, false)
            }
            mir::Type::Closure { signature, .. } => {
                let signature = signature.ty().ok_or(Error::invalid_instruction())?;

                (signature, true)
            }
            _ => return Err(Error::invalid_instruction()),
        };

        let offset = word_offset(self, callee)?;

        Ok(IndirectCallee {
            offset,
            signature,
            has_environment,
        })
    }

    /// Lower one direct call terminator.
    fn lower_call_branch(
        &self,
        function: mir::FunctionReference,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let function = function
            .function()
            .ok_or_else(|| Error::invalid_program("call callee"))?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "call argument")?;
        let target_state = self.call_target_state()?;

        Ok(pool.instruction_with_side(
            Op::CallBranch,
            CallBranch {
                function: function.id,
                target: self.call_target(function)?,
                arguments,
                target_state,
            },
        ))
    }

    /// Lower one indirect call terminator.
    fn lower_indirect_call_branch(
        &self,
        callee: mir::ValueReference,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let callee = callee
            .value()
            .ok_or_else(|| Error::invalid_program("call indirect callee"))?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "call indirect argument")?;
        let target_state = self.call_target_state()?;
        let callee = self.indirect_callee(callee)?;

        Ok(pool.instruction_with_side(
            indirect_call_branch_op(&callee),
            CallIndirectBranch {
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
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::invalid_program("call virtual receiver"))?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "call virtual argument")?;
        let target_state = self.call_target_state()?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = virtual_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("call virtual table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            virtual_call_branch_op(pointer_class)?,
            CallVirtualBranch {
                receiver_offset: word_offset(self, receiver)?,
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
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::invalid_program("call dynamic receiver"))?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "call dynamic argument")?;
        let target_state = self.call_target_state()?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = dynamic_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("call dynamic table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            dynamic_call_branch_op(pointer_class)?,
            CallDynamicBranch {
                receiver_offset: word_offset(self, receiver)?,
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
        function: mir::FunctionReference,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let function = function
            .function()
            .ok_or_else(|| Error::invalid_program("tail call callee"))?;
        if function == self.function_id {
            let arguments =
                pool.argument_reference_range(call.arguments.as_slice(), "tail call argument")?;

            return Ok(Instruction::new(
                Op::TailCallSelf,
                self.entry_block,
                arguments.start,
                arguments.len,
                0,
            ));
        }

        let callee = self.tree.get(function);
        let arguments = call
            .arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::invalid_program("tail call argument"))
            })
            .collect::<Result<Vec<_>>>()?;
        let moves = pool.parameter_move_range(&callee.parameters, &arguments)?;
        let target = self.call_target(function)?;

        Ok(pool.instruction_with_side(
            Op::TailCall,
            TailCall {
                function: function.id,
                target,
                moves,
            },
        ))
    }

    /// Lower one indirect tail call.
    fn lower_indirect_tail_call(
        &self,
        callee: mir::ValueReference,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let callee = callee
            .value()
            .ok_or_else(|| Error::invalid_program("tail indirect callee"))?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail indirect argument")?;
        let callee = self.indirect_callee(callee)?;

        Ok(pool.instruction_with_side(
            indirect_tail_call_op(&callee),
            TailCallIndirect {
                callee_offset: callee.offset,
                signature: callee.signature,
                arguments,
            },
        ))
    }

    /// Lower one virtual tail call.
    fn lower_virtual_tail_call(
        &self,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::invalid_program("tail virtual receiver"))?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail virtual argument")?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = virtual_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value_layout(self.value_layout_map(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("tail virtual table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            virtual_tail_call_op(pointer_class)?,
            TailCallVirtual {
                receiver_offset: word_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Lower one dynamic tail call.
    fn lower_dynamic_tail_call(
        &self,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::invalid_program("tail dynamic receiver"))?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail dynamic argument")?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = dynamic_table_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value_layout(self.value_layout_map(), receiver),
        )
        .ok_or_else(|| Error::invalid_program("tail dynamic table field"))?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            dynamic_tail_call_op(pointer_class)?,
            TailCallDynamic {
                receiver_offset: word_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }
}

/// Static callee shape for one indirect call.
struct IndirectCallee {
    /// Word offset of the closure value in the current frame.
    offset: u32,
    /// Expected function signature.
    signature: mir::LocalNodeId<mir::Type>,
    /// Whether the closure carries an environment pointer.
    has_environment: bool,
}

/// Return the virtual call op for one receiver pointer class.
fn virtual_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::CallVirtualHeap),
        PointerClass::SharedHeap => Ok(Op::CallVirtualSharedHeap),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Return the virtual call terminator op for one receiver pointer class.
fn virtual_call_branch_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::CallVirtualHeapBranch),
        PointerClass::SharedHeap => Ok(Op::CallVirtualSharedHeapBranch),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Return the virtual tail call op for one receiver pointer class.
fn virtual_tail_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::TailCallVirtualHeap),
        PointerClass::SharedHeap => Ok(Op::TailCallVirtualSharedHeap),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Return the dynamic call op for one receiver pointer class.
fn dynamic_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::CallDynamicHeap),
        PointerClass::SharedHeap => Ok(Op::CallDynamicSharedHeap),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Return the dynamic call terminator op for one receiver pointer class.
fn dynamic_call_branch_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::CallDynamicHeapBranch),
        PointerClass::SharedHeap => Ok(Op::CallDynamicSharedHeapBranch),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Return the dynamic tail call op for one receiver pointer class.
fn dynamic_tail_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::TailCallDynamicHeap),
        PointerClass::SharedHeap => Ok(Op::TailCallDynamicSharedHeap),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Select one indirect call opcode from callee shape.
fn indirect_call_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::CallClosure;
    }

    Op::CallIndirect
}

/// Select one indirect call terminator opcode from callee shape.
fn indirect_call_branch_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::CallClosureBranch;
    }

    Op::CallIndirectBranch
}

/// Select one indirect tail call opcode from callee shape.
fn indirect_tail_call_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::TailCallClosure;
    }

    Op::TailCallIndirect
}
