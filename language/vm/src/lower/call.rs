use destack_mir as mir;

use crate::program::{Instruction, Opcode, Operands, pack_optional_value};
use crate::{Error, Result};

use super::access::{interface_table_field, virtual_table_field};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::value::{heap_pointee_type_for_value, pointer_class_for_value};

impl<'a> BlockLowerer<'a> {
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

        Ok(Instruction {
            opcode: Opcode::Call,
            operands: Operands::Call {
                dest: self.optional_value(destination, "call destination")?,
                function: function.id,
                target,
                arguments: argument_range,
                moves,
            },
        })
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

        Ok(Instruction {
            opcode: Opcode::CallVirtual,
            operands: Operands::CallVirtual {
                dest: self.optional_value(destination, "virtual call destination")?,
                receiver,
                table_field,
                method_index: method.0,
                arguments,
            },
        })
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

        Ok(Instruction {
            opcode: Opcode::CallInterface,
            operands: Operands::CallInterface {
                dest: self.optional_value(destination, "interface call destination")?,
                receiver,
                table_field,
                method_index: method.0,
                arguments,
            },
        })
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

        Ok(Instruction {
            opcode: Opcode::CallIndirect,
            operands: Operands::CallIndirect {
                dest: self.optional_value(destination, "indirect call destination")?,
                callee,
                arguments,
            },
        })
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

        Ok(Instruction {
            opcode: Opcode::BindCallable,
            operands: Operands::CallableBind {
                dest: destination,
                function: function.id,
                environment,
            },
        })
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

        Ok(Instruction {
            opcode: Opcode::LoadCallableEnvironment,
            operands: Operands::CallableEnvironment { dest: destination },
        })
    }

    /// Pack an optional MIR value for a call destination.
    fn optional_value(
        &self,
        value: Option<mir::ValueReference>,
        context: &'static str,
    ) -> Result<mir::Value> {
        Ok(pack_optional_value(
            value
                .map(|value| {
                    value.value().ok_or_else(|| Error::MissingRepresentation {
                        context: context.into(),
                    })
                })
                .transpose()?,
        ))
    }
}
