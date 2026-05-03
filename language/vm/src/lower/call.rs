use {destack_engine as engine, destack_mir as mir};

use crate::program::{
    Call, CallBranch, CallIndirect, CallIndirectBranch, CallInterface, CallInterfaceBranch,
    CallVirtual, CallVirtualBranch, CallableBind, CallableEnvironment, Instruction, Opcode,
    TailCall, TailCallIndirect, TailCallInterface, TailCallSelf, TailCallVirtual,
};
use crate::{Error, Result};

use super::access::{interface_table_field, virtual_table_field};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::value::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_layout, pointer_class_for_value,
};

impl<'a> BlockLowerer<'a> {
    /// Lower one call-like terminator.
    pub(super) fn lower_call_terminator(
        &self,
        term: &mir::Terminator,
        pool: &mut Pool<'_>,
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
                slot_id,
                call,
                ..
            } => self.lower_virtual_invoke(*receiver, *slot_id, call, pool)?,
            mir::Terminator::InvokeInterface {
                receiver,
                slot_id,
                call,
                ..
            } => self.lower_interface_invoke(*receiver, *slot_id, call, pool)?,
            mir::Terminator::TailCall { function, call, .. } => {
                self.lower_tail_call(*function, call, pool)?
            }
            mir::Terminator::TailCallIndirect { callee, call, .. } => {
                self.lower_indirect_tail_call(*callee, call, pool)?
            }
            mir::Terminator::TailCallVirtual {
                receiver,
                slot_id,
                call,
                ..
            } => self.lower_virtual_tail_call(*receiver, *slot_id, call, pool)?,
            mir::Terminator::TailCallInterface {
                receiver,
                slot_id,
                call,
                ..
            } => self.lower_interface_tail_call(*receiver, *slot_id, call, pool)?,
            _ => return Err(Error::InvalidInstruction),
        })
    }

    /// Lower one direct call.
    pub(super) fn lower_call(
        &self,
        destination: Option<mir::ValueReference>,
        function: mir::FunctionReference,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "call callee".to_string(),
            })?;
        let arguments = self.tree.get_arguments(call.arguments);
        let argument_range = pool.argument_reference_range(arguments, "call argument")?;
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

        Ok(Instruction::new(
            Opcode::Call,
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
        method: mir::VtableSlotId,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "virtual call argument",
        )?;
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "virtual call receiver".to_string(),
            })?;
        let table_field = virtual_table_field(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
            pointer_class_for_value(self.value_layout_map(), receiver),
        )
        .map(|field| pool.field_access(field));

        Ok(Instruction::new(
            Opcode::CallVirtual,
            CallVirtual {
                dest: self.optional_value(destination, "virtual call destination")?,
                receiver,
                table_field,
                method_index: method.0,
                arguments,
            },
        ))
    }

    /// Lower one interface call.
    pub(super) fn lower_interface_call(
        &self,
        destination: Option<mir::ValueReference>,
        receiver: mir::ValueReference,
        method: mir::InterfaceSlotId,
        call: &mir::Call<mir::ArgumentSlice>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "interface call argument",
        )?;
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "interface call receiver".to_string(),
            })?;
        let table_field = interface_table_field(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
            pointer_class_for_value(self.value_layout_map(), receiver),
        )
        .map(|field| pool.field_access(field));

        Ok(Instruction::new(
            Opcode::CallInterface,
            CallInterface {
                dest: self.optional_value(destination, "interface call destination")?,
                receiver,
                table_field,
                method_index: method.0,
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
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let arguments = pool.argument_reference_range(
            self.tree.get_arguments(call.arguments),
            "indirect call argument",
        )?;
        let callee = callee.value().ok_or_else(|| Error::MissingRepresentation {
            context: "indirect call callee".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::CallIndirect,
            CallIndirect {
                dest: self.optional_value(destination, "indirect call destination")?,
                callee,
                arguments,
            },
        ))
    }

    /// Lower one callable binding.
    pub(super) fn lower_callable_bind(
        &self,
        destination: mir::ValueReference,
        function: mir::FunctionReference,
        environment: mir::ValueReference,
    ) -> Result<Instruction> {
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

        Ok(Instruction::new(
            Opcode::BindCallable,
            CallableBind {
                dest: destination,
                function: function.id,
                environment,
            },
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
            Opcode::LoadCallableEnvironment,
            CallableEnvironment { dest: destination },
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

    /// Lower one direct exceptional call.
    fn lower_invoke(
        &self,
        function: mir::FunctionReference,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "invoke callee".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;

        Ok(Instruction::new(
            Opcode::Invoke,
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
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let callee = callee.value().ok_or_else(|| Error::MissingRepresentation {
            context: "invoke indirect callee".to_string(),
        })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke indirect argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;

        Ok(Instruction::new(
            Opcode::InvokeIndirect,
            CallIndirectBranch {
                callee,
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
        method: mir::VtableSlotId,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "invoke virtual receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke virtual argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;
        let table_field = virtual_table_field(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
            pointer_class_for_value(self.value_layout_map(), receiver),
        )
        .map(|field| pool.field_access(field));

        Ok(Instruction::new(
            Opcode::InvokeVirtual,
            CallVirtualBranch {
                receiver,
                table_field,
                method_index: method.0,
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
        method: mir::InterfaceSlotId,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "invoke interface receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "invoke interface argument")?;
        let (normal_state, unwind_state) = self.exceptional_call_states()?;
        let table_field = interface_table_field(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
            pointer_class_for_value(self.value_layout_map(), receiver),
        )
        .map(|field| pool.field_access(field));

        Ok(Instruction::new(
            Opcode::InvokeInterface,
            CallInterfaceBranch {
                receiver,
                table_field,
                method_index: method.0,
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
        pool: &mut Pool<'_>,
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
                Opcode::TailCallSelf,
                TailCallSelf {
                    entry: self.entry_block,
                    arguments,
                },
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

        Ok(Instruction::new(
            Opcode::TailCall,
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
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let callee = callee.value().ok_or_else(|| Error::MissingRepresentation {
            context: "tail indirect callee".to_string(),
        })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail indirect argument")?;

        Ok(Instruction::new(
            Opcode::TailCallIndirect,
            TailCallIndirect { callee, arguments },
        ))
    }

    /// Lower one virtual tail call.
    fn lower_virtual_tail_call(
        &self,
        receiver: mir::ValueReference,
        method: mir::VtableSlotId,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "tail virtual receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail virtual argument")?;
        let table_field = virtual_table_field(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value_layout(self.value_layout_map(), receiver),
            pointer_class_for_value(self.value_layout_map(), receiver),
        )
        .map(|field| pool.field_access(field));

        Ok(Instruction::new(
            Opcode::TailCallVirtual,
            TailCallVirtual {
                receiver,
                table_field,
                method_index: method.0,
                arguments,
            },
        ))
    }

    /// Lower one interface tail call.
    fn lower_interface_tail_call(
        &self,
        receiver: mir::ValueReference,
        method: mir::InterfaceSlotId,
        call: &mir::Call<Vec<mir::ValueReference>>,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let receiver = receiver
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "tail interface receiver".to_string(),
            })?;
        let arguments =
            pool.argument_reference_range(call.arguments.as_slice(), "tail interface argument")?;
        let table_field = interface_table_field(
            self.tree,
            self.layouts(),
            heap_pointee_type_for_value_layout(self.value_layout_map(), receiver),
            pointer_class_for_value(self.value_layout_map(), receiver),
        )
        .map(|field| pool.field_access(field));

        Ok(Instruction::new(
            Opcode::TailCallInterface,
            TailCallInterface {
                receiver,
                table_field,
                method_index: method.0,
                arguments,
            },
        ))
    }
}
