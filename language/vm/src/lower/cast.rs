use destack_mir as mir;

use crate::program::{
    CastWideInt, CastWideIntToWord, CastWord, CastWordToWideInt, Instruction, Opcode, SelectFrame,
    SelectWord, ValueLayout, value_layout_from_type,
};
use crate::{Error, Result};

use super::frame::word_offset;
use super::lower::BlockLowerer;

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
                Opcode::CastWord,
                CastWord {
                    dest: word_offset(self, destination)?,
                    op: operator,
                    arg: word_offset(self, argument)?,
                    to_type: to_type.id,
                },
            ));
        }

        // wide integer casts need explicit source and destination widths
        let (source_width, source_signed) = integer_layout(self.tree, argument_type)?;
        let (dest_width, dest_signed) = integer_layout(self.tree, to_type)?;
        let source_signed = matches!(operator, mir::CastOperator::SignExtend) || source_signed;

        // expand a word into frame bytes
        if argument_is_word {
            return Ok(Instruction::new(
                Opcode::CastWordToWideInt,
                CastWordToWideInt {
                    dest: destination,
                    op: operator,
                    arg: word_offset(self, argument)?,
                    source_width,
                    source_signed,
                    dest_width,
                },
            ));
        }

        // collapse frame bytes into a word
        if destination_is_word {
            return Ok(Instruction::new(
                Opcode::CastWideIntToWord,
                CastWideIntToWord {
                    dest: word_offset(self, destination)?,
                    op: operator,
                    arg: argument,
                    source_width,
                    source_signed,
                    dest_width,
                    dest_signed,
                },
            ));
        }

        // transform wide integer frame bytes
        Ok(Instruction::new(
            Opcode::CastWideInt,
            CastWideInt {
                dest: destination,
                op: operator,
                arg: argument,
                source_width,
                source_signed,
                dest_width,
            },
        ))
    }

    /// Lower one select instruction.
    pub(super) fn lower_select(
        &self,
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
                Opcode::SelectWord,
                SelectWord {
                    dest: word_offset(self, destination)?,
                    condition: word_offset(self, condition)?,
                    then_value: word_offset(self, then_value)?,
                    else_value: word_offset(self, else_value)?,
                },
            ));
        }

        // select frame-backed values by copying their frame region
        Ok(Instruction::new(
            Opcode::SelectFrame,
            SelectFrame {
                dest: destination,
                condition: word_offset(self, condition)?,
                then_value,
                else_value,
            },
        ))
    }
}
