use destack_mir as mir;

use crate::program::{Instruction, Op, ValueLayout, value_layout_from_type};
use crate::{Error, Result};

use super::frame::word_offset;
use super::lower::BlockLowerer;
use super::pool::Pool;

/// Return one cast operator operand.
fn cast_operator_operand(operator: mir::CastOperator) -> u32 {
    operator as u32
}

/// Return one wide integer cast operator operand.
fn wide_cast_operator_operand(
    operator: mir::CastOperator,
    source_signed: bool,
    dest_signed: bool,
) -> u32 {
    let source_signed = u32::from(source_signed) << 8;
    let dest_signed = u32::from(dest_signed) << 9;

    cast_operator_operand(operator) | source_signed | dest_signed
}

/// Return one wide integer cast layout operand.
fn wide_cast_layout_operand(source_width: u16, dest_width: u16) -> u32 {
    u32::from(source_width) | (u32::from(dest_width) << 16)
}

/// Return one integer value layout.
fn integer_layout(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<(u16, bool)> {
    match value_layout_from_type(tree, ty) {
        ValueLayout::Int { width, signed } => Ok((width, signed)),
        actual => Err(Error::TypeMismatch {
            expected: "integer cast operand".to_string(),
            actual: format!("{actual:?}"),
        }),
    }
}

impl<'a> BlockLowerer<'a> {
    /// Lower one cast instruction.
    pub(super) fn lower_cast(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        operator: mir::CastOperator,
        argument: mir::ValueReference,
        to_type: mir::TypeReference,
    ) -> Result<Instruction> {
        // require SSA values and the target type
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
        let destination_type = self.value_type_for_value(destination)?;
        let argument_type = self.value_type_for_value(argument)?;
        let destination_is_word = self.layout_for_type(destination_type)?.is_word();
        let argument_is_word = self.layout_for_type(argument_type)?.is_word();

        // use the direct word path when both sides fit in one word
        if destination_is_word && argument_is_word {
            return Ok(Instruction::new(
                Op::CastWord,
                word_offset(self, destination)?,
                cast_operator_operand(operator),
                word_offset(self, argument)?,
                to_type.id,
            ));
        }

        // wide integer casts need explicit source and destination widths
        let (source_width, source_signed) = integer_layout(self.tree, argument_type)?;
        let (dest_width, dest_signed) = integer_layout(self.tree, to_type)?;
        let source_signed = matches!(operator, mir::CastOperator::SignExtend) || source_signed;

        // expand a word into frame bytes
        if argument_is_word {
            return Ok(Instruction::new(
                Op::CastWordToWideInt,
                destination.id(),
                word_offset(self, argument)?,
                wide_cast_operator_operand(operator, source_signed, false),
                wide_cast_layout_operand(source_width, dest_width),
            ));
        }

        // collapse frame bytes into a word
        if destination_is_word {
            return Ok(Instruction::new(
                Op::CastWideIntToWord,
                word_offset(self, destination)?,
                argument.id(),
                wide_cast_operator_operand(operator, source_signed, dest_signed),
                wide_cast_layout_operand(source_width, dest_width),
            ));
        }

        // transform wide integer frame bytes
        Ok(Instruction::new(
            Op::CastWideInt,
            destination.id(),
            argument.id(),
            wide_cast_operator_operand(operator, source_signed, false),
            wide_cast_layout_operand(source_width, dest_width),
        ))
    }

    /// Lower one select instruction.
    pub(super) fn lower_select(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        condition: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
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

        // select word values without touching frame bytes
        let destination_type = self.value_type_for_value(destination)?;
        if self.layout_for_type(destination_type)?.is_word() {
            return Ok(Instruction::new(
                Op::SelectWord,
                word_offset(self, destination)?,
                word_offset(self, condition)?,
                word_offset(self, then_value)?,
                word_offset(self, else_value)?,
            ));
        }

        // select frame-backed values by copying their frame slot
        Ok(Instruction::new(
            Op::SelectFrame,
            destination.id(),
            word_offset(self, condition)?,
            then_value.id(),
            else_value.id(),
        ))
    }
}
