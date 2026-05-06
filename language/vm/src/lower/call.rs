use {destack_engine as engine, destack_mir as mir};

use crate::program::{
    Call, CallBranch, CallIndirect, CallIndirectBranch, CallInterface, CallInterfaceBranch,
    CallVirtual, CallVirtualBranch, CallableBind, CallableEnvironment, Instruction, Op,
    PointerClass, TailCall, TailCallIndirect, TailCallInterface, TailCallVirtual,
    callable_object_layout, repr_type, word_layout_from_type,
};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::projection::{itab_projection, vtable_projection};
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
            mir::Terminator::Invoke { function, call, .. } => {
                self.lower_invoke(*function, call, pool)?
            }
            mir::Terminator::InvokeIndirect { callee, call, .. } => {
                self.lower_indirect_invoke(*callee, call, pool)?
            }
            mir::Terminator::InvokeVirtual {
                receiver,
                slot,
                call,
                ..
            } => self.lower_virtual_invoke(*receiver, *slot, call, pool)?,
            mir::Terminator::InvokeInterface {
                receiver,
                slot,
                call,
                ..
            } => self.lower_interface_invoke(*receiver, *slot, call, pool)?,
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
            mir::Terminator::TailCallInterface {
                receiver,
                slot,
                call,
                ..
            } => self.lower_interface_tail_call(*receiver, *slot, call, pool)?,
            _ => return Err(Error::InvalidInstruction),
        })
    }

    /// Lower one direct call.
    pub(super) fn lower_call(
        &self,
        destination: Option<mir::ValueReference>,
        function: mir::FunctionReference,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve callee and arguments
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "call callee".to_string(),
            })?;
        let arguments = self.tree.get_arguments(call.arguments);
        let argument_range = pool.argument_reference_range(arguments, "call argument")?;

        // compute frame moves once during lowering
        let argument_value = arguments
            .iter()
            .map(|argument| {
                argument
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "call argument".to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let callee = self.tree.get(function);
        let moves = pool.parameter_move_range(&callee.parameters, &argument_value)?;
        let target = self.call_target(function)?;

        // emit the compact call instruction
        Ok(pool.instruction_with_side(
            Op::Call,
            Call {
                dest: self.optional_value(destination, "call destination")?,
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
        destination: Option<mir::ValueReference>,
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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "virtual call receiver".to_string(),
            })?;

        // compile the receiver table access
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = vtable_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::MissingRepresentation {
            context: "virtual call table field".to_string(),
        })?;
        let table_field = pool.projection(table_field);

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            virtual_call_op(pointer_class)?,
            CallVirtual {
                dest: self.optional_value(destination, "virtual call destination")?,
                receiver_offset: word_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
            },
        ))
    }

    /// Lower one interface call.
    pub(super) fn lower_interface_call(
        &self,
        destination: Option<mir::ValueReference>,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve arguments and receiver
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "interface call argument",
        )?;
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "interface call receiver".to_string(),
            })?;

        // compile the receiver table access
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = itab_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::MissingRepresentation {
            context: "interface call table field".to_string(),
        })?;
        let table_field = pool.projection(table_field);

        // emit the receiver-space-specific opcode
        Ok(pool.instruction_with_side(
            interface_call_op(pointer_class)?,
            CallInterface {
                dest: self.optional_value(destination, "interface call destination")?,
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
        destination: Option<mir::ValueReference>,
        callee: mir::ValueReference,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // resolve call arguments
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "indirect call argument",
        )?;

        // resolve callable shape
        let callee = callee.value().ok_or_else(|| Error::MissingRepresentation {
            context: "indirect call callee".to_string(),
        })?;
        let callee = self.indirect_callee(callee)?;

        // emit the function-pointer or callable opcode
        Ok(pool.instruction_with_side(
            indirect_call_op(&callee),
            CallIndirect {
                dest: self.optional_value(destination, "indirect call destination")?,
                callee_offset: callee.offset,
                signature: callee.signature,
                arguments,
            },
        ))
    }

    /// Lower one callable binding.
    pub(super) fn lower_callable_bind(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        function: mir::FunctionReference,
        environment: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve callable operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "callable bind destination".to_string(),
            })?;
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "callable bind callee".to_string(),
            })?;
        let environment = environment
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "callable bind environment".to_string(),
            })?;

        // select the environment representation
        let destination_type = self.value_type_for_value(destination)?;
        let environment_type = self.value_type_for_value(environment)?;
        let environment_layout = self.layout_for_type(environment_type)?;

        let (op, environment_repr) = if environment_layout.is_word() {
            let layout = word_layout_from_type(self.tree, environment_type)
                .ok_or(Error::InvalidInstruction)?;

            (Op::BindCallableWord, CallableEnvironment::Word { layout })
        } else {
            (
                Op::BindCallableAddress,
                CallableEnvironment::Frame {
                    layout: self.layout_id_for_type(environment_type)?,
                    byte_len: environment_layout.byte_len,
                },
            )
        };

        // pool the cold callable layout metadata
        let callable = CallableBind {
            callable_layout: self.layout_id_for_type(destination_type)?,
            object_layout: callable_object_layout(self.tree.pointer_bytes() as usize),
            environment: environment_repr,
        };
        let callable = pool.side_record(callable);

        // put the hot operands in the instruction payload
        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            function.id,
            value_offset(self, environment)?,
            callable,
        ))
    }

    /// Lower one callable environment read.
    pub(super) fn lower_callable_environment(
        &self,
        destination: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "callable environment destination".to_string(),
            })?;

        Ok(Instruction::new(
            Op::LoadCallableEnvironment,
            word_offset(self, destination)?,
            0,
            0,
            0,
        ))
    }

    /// Resolve an optional MIR value for a call destination.
    fn optional_value(
        &self,
        value: Option<mir::ValueReference>,
        context: &'static str,
    ) -> Result<Option<mir::Value>> {
        value
            .map(|value| {
                value.value().ok_or_else(|| Error::MissingRepresentation {
                    context: context.to_string(),
                })
            })
            .transpose()
    }

    /// Return the normal and unwind frame states for an exceptional call.
    fn exceptional_call_states(&self) -> Result<(engine::FrameStateId, engine::FrameStateId)> {
        self.exceptional_call_frame_states
            .get(&self.block_id())
            .copied()
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing exceptional call frame states for block: {:?}",
                    self.block_id()
                ),
            })
    }

    /// Return static callee shape for one indirect call.
    fn indirect_callee(&self, callee: mir::Value) -> Result<IndirectCallee> {
        let callee_type = self.value_type_for_value(callee)?;
        let callee_type = repr_type(self.tree, callee_type);

        // function pointers carry only the target function id
        let (signature, has_environment) = match self.tree.get(callee_type) {
            mir::Type::FunctionPointer { signature } => {
                let signature = signature.ty().ok_or(Error::InvalidInstruction)?;

                (signature, false)
            }
            mir::Type::Callable { signature } => {
                let signature = signature.ty().ok_or(Error::InvalidInstruction)?;

                (signature, true)
            }
            _ => return Err(Error::InvalidInstruction),
        };

        let offset = word_offset(self, callee)?;

        Ok(IndirectCallee {
            offset,
            signature,
            has_environment,
        })
    }

    /// Lower one direct exceptional call.
    fn lower_invoke(
        &self,
        function: mir::FunctionReference,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "invoke callee".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;

        Ok(pool.instruction_with_side(
            Op::Invoke,
            CallBranch {
                function: function.id,
                target: self.call_target(function)?,
                arguments,
                normal_state,
                unwind_state,
            },
        ))
    }

    /// Lower one indirect exceptional call.
    fn lower_indirect_invoke(
        &self,
        callee: mir::ValueReference,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let callee = callee.value().ok_or_else(|| Error::MissingRepresentation {
            context: "invoke indirect callee".to_string(),
        })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke indirect argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;
        let callee = self.indirect_callee(callee)?;

        Ok(pool.instruction_with_side(
            indirect_invoke_op(&callee),
            CallIndirectBranch {
                callee_offset: callee.offset,
                signature: callee.signature,
                arguments,
                normal_state,
                unwind_state,
            },
        ))
    }

    /// Lower one virtual exceptional call.
    fn lower_virtual_invoke(
        &self,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "invoke virtual receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke virtual argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = vtable_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::MissingRepresentation {
            context: "invoke virtual table field".to_string(),
        })?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            virtual_invoke_op(pointer_class)?,
            CallVirtualBranch {
                receiver_offset: word_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
                normal_state,
                unwind_state,
            },
        ))
    }

    /// Lower one interface exceptional call.
    fn lower_interface_invoke(
        &self,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "invoke interface receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke interface argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = itab_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
        )
        .ok_or_else(|| Error::MissingRepresentation {
            context: "invoke interface table field".to_string(),
        })?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            interface_invoke_op(pointer_class)?,
            CallInterfaceBranch {
                receiver_offset: word_offset(self, receiver)?,
                table_field,
                slot: method.0,
                arguments,
                normal_state,
                unwind_state,
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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "tail call callee".to_string(),
            })?;
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
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "tail call argument".to_string(),
                    })
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
        let callee = callee.value().ok_or_else(|| Error::MissingRepresentation {
            context: "tail indirect callee".to_string(),
        })?;
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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "tail virtual receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail virtual argument")?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = vtable_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value_layout(self.value_layout_map(), receiver),
        )
        .ok_or_else(|| Error::MissingRepresentation {
            context: "tail virtual table field".to_string(),
        })?;
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

    /// Lower one interface tail call.
    fn lower_interface_tail_call(
        &self,
        receiver: mir::ValueReference,
        method: mir::DispatchSlot,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "tail interface receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail interface argument")?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), receiver);
        let table_field = itab_projection(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value_layout(self.value_layout_map(), receiver),
        )
        .ok_or_else(|| Error::MissingRepresentation {
            context: "tail interface table field".to_string(),
        })?;
        let table_field = pool.projection(table_field);

        Ok(pool.instruction_with_side(
            interface_tail_call_op(pointer_class)?,
            TailCallInterface {
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
    /// Word offset of the callable value in the current frame.
    offset: u32,
    /// Expected function signature.
    signature: mir::LocalNodeId<mir::Type>,
    /// Whether the callable carries an environment pointer.
    has_environment: bool,
}

/// Return the virtual call op for one receiver pointer class.
fn virtual_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::CallVirtualHeap),
        PointerClass::SharedHeap => Ok(Op::CallVirtualSharedHeap),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Return the virtual invoke op for one receiver pointer class.
fn virtual_invoke_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::InvokeVirtualHeap),
        PointerClass::SharedHeap => Ok(Op::InvokeVirtualSharedHeap),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Return the virtual tail call op for one receiver pointer class.
fn virtual_tail_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::TailCallVirtualHeap),
        PointerClass::SharedHeap => Ok(Op::TailCallVirtualSharedHeap),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Return the interface call op for one receiver pointer class.
fn interface_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::CallInterfaceHeap),
        PointerClass::SharedHeap => Ok(Op::CallInterfaceSharedHeap),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Return the interface invoke op for one receiver pointer class.
fn interface_invoke_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::InvokeInterfaceHeap),
        PointerClass::SharedHeap => Ok(Op::InvokeInterfaceSharedHeap),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Return the interface tail call op for one receiver pointer class.
fn interface_tail_call_op(pointer_class: PointerClass) -> Result<Op> {
    match pointer_class {
        PointerClass::Heap => Ok(Op::TailCallInterfaceHeap),
        PointerClass::SharedHeap => Ok(Op::TailCallInterfaceSharedHeap),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Select one indirect call opcode from callee shape.
fn indirect_call_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::CallCallable;
    }

    Op::CallIndirect
}

/// Select one exceptional indirect call opcode from callee shape.
fn indirect_invoke_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::InvokeCallable;
    }

    Op::InvokeIndirect
}

/// Select one indirect tail call opcode from callee shape.
fn indirect_tail_call_op(callee: &IndirectCallee) -> Op {
    if callee.has_environment {
        return Op::TailCallCallable;
    }

    Op::TailCallIndirect
}
