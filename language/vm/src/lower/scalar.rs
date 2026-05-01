use destack_mir as mir;

use crate::program::{
    ConstValue, Instruction, Opcode, Operands, ValueLayout, value_layout_from_type,
};
use crate::{Error, ReferenceAddressSpace, Result, Word};

use super::lower::BlockLowerer;
use super::opcode::{
    select_binary_opcode, select_integer_opcode, select_integer_unary_opcode, select_unary_opcode,
};
use super::value::reference_meta_for_type;

/// Encode a constant into its frame bytes.
fn constant_bytes(value: &mir::Constant, byte_len: usize) -> Result<Box<[u8]>> {
    let mut bytes = vec![0; byte_len];

    match value {
        mir::Constant::Int {
            value,
            is_signed: true,
            ..
        } => {
            let source = value.to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);

            if *value < 0 && byte_len > source.len() {
                bytes[source.len()..].fill(0xff);
            }
        }
        mir::Constant::Int { value, .. } => {
            let source = (*value as u128).to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        mir::Constant::UInt { value, .. } => {
            let source = value.to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        mir::Constant::Float { bits: value, .. } => {
            let source = value.to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: "frame-backed constant".to_string(),
                actual: format!("{value:?}"),
            });
        }
    }

    Ok(bytes.into_boxed_slice())
}

impl<'a> BlockLowerer<'a> {
    /// Lower one constant instruction.
    pub(super) fn lower_const(
        &self,
        destination: mir::ValueReference,
        value: &mir::Constant,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "const destination".to_string(),
            })?;
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;

        let value = if matches!(value, mir::Constant::Null) {
            ConstValue::Word(self.null_word(destination_type))
        } else if layout.is_word() {
            ConstValue::Word(Word::from(value))
        } else {
            ConstValue::Bytes(constant_bytes(value, layout.byte_len)?)
        };

        Ok(Instruction {
            opcode: Opcode::LoadConst,
            operands: Operands::LoadConst {
                dest: destination,
                value,
            },
        })
    }

    /// Lower one binary instruction.
    pub(super) fn lower_binary(
        &self,
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "binary destination".to_string(),
            })?;
        let left = left.value().ok_or_else(|| Error::MissingRepresentation {
            context: "binary left operand".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "binary right operand".to_string(),
        })?;
        let left_type = self.value_type_for_value(left)?;

        if matches!(
            self.tree.get(left_type),
            mir::Type::Vector { .. } | mir::Type::Tensor { .. }
        ) {
            let result_type = self.value_type_for_value(destination)?;
            return Ok(Instruction {
                opcode: Opcode::BinaryElementwise,
                operands: Operands::BinaryElementwise {
                    dest: destination,
                    op: operator,
                    left,
                    right,
                    result_type,
                },
            });
        }

        let layout = self.value_layout_map().get(left);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(opcode) = select_integer_opcode(operator, signed, width)
        {
            return Ok(Instruction {
                opcode,
                operands: Operands::BinaryInteger {
                    dest: destination,
                    left,
                    right,
                    width: width as u8,
                    is_signed: signed,
                },
            });
        }

        let layout = layout.or_else(|| Some(value_layout_from_type(self.tree, left_type)));
        Ok(Instruction {
            opcode: select_binary_opcode(layout, operator),
            operands: Operands::Binary {
                dest: destination,
                op: operator,
                left,
                right,
            },
        })
    }

    /// Lower one unary instruction.
    pub(super) fn lower_unary(
        &self,
        destination: mir::ValueReference,
        operator: mir::UnaryOperator,
        argument: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "unary destination".to_string(),
            })?;
        let argument = argument
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "unary argument".to_string(),
            })?;
        let argument_type = self.value_type_for_value(argument)?;

        if matches!(
            self.tree.get(argument_type),
            mir::Type::Vector { .. } | mir::Type::Tensor { .. }
        ) {
            let result_type = self.value_type_for_value(destination)?;
            return Ok(Instruction {
                opcode: Opcode::UnaryElementwise,
                operands: Operands::UnaryElementwise {
                    dest: destination,
                    op: operator,
                    arg: argument,
                    result_type,
                },
            });
        }

        let layout = self.value_layout_map().get(argument);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(opcode) = select_integer_unary_opcode(operator, signed, width)
        {
            return Ok(Instruction {
                opcode,
                operands: Operands::UnaryInteger {
                    dest: destination,
                    arg: argument,
                    width: width as u8,
                    is_signed: signed,
                },
            });
        }

        Ok(Instruction {
            opcode: select_unary_opcode(self.value_layout_map(), argument, operator),
            operands: Operands::Unary {
                dest: destination,
                op: operator,
                arg: argument,
            },
        })
    }

    /// Lower one cast instruction.
    pub(super) fn lower_cast(
        &self,
        destination: mir::ValueReference,
        operator: mir::CastOperator,
        argument: mir::ValueReference,
        to_type: mir::TypeReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "cast destination".to_string(),
            })?;
        let argument = argument
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "cast argument".to_string(),
            })?;
        let to_type = to_type.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "cast destination type".to_string(),
        })?;

        Ok(Instruction {
            opcode: Opcode::Cast,
            operands: Operands::Cast {
                dest: destination,
                op: operator,
                arg: argument,
                to_type: to_type.id,
            },
        })
    }

    /// Lower one select instruction.
    pub(super) fn lower_select(
        &self,
        destination: mir::ValueReference,
        condition: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select destination".to_string(),
            })?;
        let condition = condition
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select condition".to_string(),
            })?;
        let then_value = then_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select then value".to_string(),
            })?;
        let else_value = else_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select else value".to_string(),
            })?;

        Ok(Instruction {
            opcode: Opcode::Select,
            operands: Operands::Select {
                dest: destination,
                condition,
                then_value,
                else_value,
            },
        })
    }

    /// Return the null word for one reference-like type.
    fn null_word(&self, value_type: mir::LocalNodeId<mir::Type>) -> Word {
        let reference = reference_meta_for_type(self.tree, value_type);
        match reference.kind() {
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned)
                if matches!(reference.address_space(), ReferenceAddressSpace::Shared) =>
            {
                Word::shared_heap_reference(destack_heap::SharedHeapReference::NULL)
            }
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned) => {
                Word::heap_reference(destack_heap::HeapReference::NULL)
            }
            _ if matches!(reference.address_space(), ReferenceAddressSpace::Shared) => {
                Word::shared_raw_pointer(destack_heap::SharedRawPointer::NULL)
            }
            _ => Word::raw_pointer(destack_heap::RawPointer::NULL),
        }
    }
}
