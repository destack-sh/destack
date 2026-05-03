use destack_mir as mir;
use destack_mir::Type;

use crate::program::{
    Binary, BinaryElementwise, BinaryInteger, BinaryWord, Instruction, Opcode, Unary,
    UnaryElementwise, UnaryInteger, UnaryWord, ValueLayout, value_layout_from_type,
};
use crate::{Error, Result};

use super::frame::word_offset;
use super::lower::BlockLowerer;
use super::opcode::{
    select_binary_opcode, select_integer_opcode, select_integer_unary_opcode, select_unary_opcode,
};

/// Build word binary operands from frame offsets.
fn binary_word(
    lowerer: &BlockLowerer<'_>,
    dest: mir::Value,
    left: mir::Value,
    right: mir::Value,
) -> Result<BinaryWord> {
    Ok(BinaryWord {
        dest: word_offset(lowerer, dest)?,
        left: word_offset(lowerer, left)?,
        right: word_offset(lowerer, right)?,
    })
}

/// Build word unary operands from frame offsets.
fn unary_word(lowerer: &BlockLowerer<'_>, dest: mir::Value, arg: mir::Value) -> Result<UnaryWord> {
    Ok(UnaryWord {
        dest: word_offset(lowerer, dest)?,
        arg: word_offset(lowerer, arg)?,
    })
}

impl<'a> BlockLowerer<'a> {
    /// Lower one binary instruction.
    pub(super) fn lower_binary(
        &self,
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
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

        // keep vector and tensor operations in their elementwise executor
        if matches!(
            self.tree.get(left_type),
            Type::Vector { .. } | Type::Tensor { .. }
        ) {
            let result_type = self.value_type_for_value(destination)?;

            return Ok(Instruction::new(
                Opcode::BinaryElementwise,
                BinaryElementwise {
                    dest: destination,
                    op: operator,
                    left,
                    right,
                    result_type,
                },
            ));
        }

        // specialize machine-word integers by signedness and width
        let layout = self.value_layout_map().get(left);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(opcode) = select_integer_opcode(operator, signed, width)
        {
            let operands = BinaryInteger {
                dest: word_offset(self, destination)?,
                left: word_offset(self, left)?,
                right: word_offset(self, right)?,
                width: width as u8,
                is_signed: signed,
            };

            return Ok(Instruction::new(opcode, operands));
        }

        // fall back to word or wide integer families
        let layout = layout.or_else(|| Some(value_layout_from_type(self.tree, left_type)));
        let opcode = select_binary_opcode(layout, operator);
        if opcode != Opcode::BinaryWideInt && opcode != Opcode::BinaryWideUint {
            let operands = binary_word(self, destination, left, right)?;

            return Ok(Instruction::new(opcode, operands));
        }

        Ok(Instruction::new(
            opcode,
            Binary {
                dest: destination,
                op: operator,
                left,
                right,
            },
        ))
    }

    /// Lower one unary instruction.
    pub(super) fn lower_unary(
        &self,
        destination: mir::ValueReference,
        operator: mir::UnaryOperator,
        argument: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
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

        // keep vector and tensor operations in their elementwise executor
        if matches!(
            self.tree.get(argument_type),
            Type::Vector { .. } | Type::Tensor { .. }
        ) {
            let result_type = self.value_type_for_value(destination)?;

            return Ok(Instruction::new(
                Opcode::UnaryElementwise,
                UnaryElementwise {
                    dest: destination,
                    op: operator,
                    arg: argument,
                    result_type,
                },
            ));
        }

        // specialize machine-word integers by signedness and width
        let layout = self.value_layout_map().get(argument);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(opcode) = select_integer_unary_opcode(operator, signed, width)
        {
            let operands = UnaryInteger {
                dest: word_offset(self, destination)?,
                arg: word_offset(self, argument)?,
                width: width as u8,
                is_signed: signed,
            };

            return Ok(Instruction::new(opcode, operands));
        }

        // fall back to word or wide integer families
        let opcode = select_unary_opcode(self.value_layout_map(), argument, operator);
        if opcode != Opcode::UnaryWideInt {
            let operands = unary_word(self, destination, argument)?;

            return Ok(Instruction::new(opcode, operands));
        }

        Ok(Instruction::new(
            opcode,
            Unary {
                dest: destination,
                op: operator,
                arg: argument,
            },
        ))
    }
}
