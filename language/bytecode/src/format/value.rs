use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, Scalar, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one register value operation.
    pub(super) fn format_value(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::MOVE | Opcode::MOVE_RANGE => self.format_move(opcode),
            Opcode::SELECT | Opcode::SELECT_RANGE => self.format_select(opcode),
            Opcode::EQUAL => self.format_equal(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid value opcode",
            }),
        }
    }

    /// Format one logical value move.
    fn format_move(&mut self, opcode: Opcode) -> FormatResult<()> {
        let (result, input, ty) = if opcode == Opcode::MOVE {
            let result = self.register_id()?;
            let input = self.register_id()?;
            let ty = self.formatter.context().register_type(input)?;

            (result, input, ty)
        } else {
            let (result, result_width) = self.register_range_id()?;
            let (input, input_width) = self.register_range_id()?;
            let ty = self.formatter.context().register_type(input)?;

            // require both physical ranges to carry one complete logical value
            if result_width != ty.word_count() || input_width != ty.word_count() {
                return Err(FormatError::SyntaxError {
                    message: "move width does not match its value type",
                });
            }

            (result, input, ty)
        };

        // write the logical move independent of its physical width
        self.write_result(result, ty)?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("move"), space()]
        )?;
        self.write_register(input)
    }

    /// Format one scalar value selection.
    fn format_select(&mut self, opcode: Opcode) -> FormatResult<()> {
        let (result, condition, left, right, ty) = if opcode == Opcode::SELECT {
            let result = self.register_id()?;
            let condition = self.register_id()?;
            let left = self.register_id()?;
            let right = self.register_id()?;
            let ty = self.formatter.context().register_type(left)?;

            (result, condition, left, right, ty)
        } else {
            let (result, result_width) = self.register_range_id()?;
            let condition = self.register_id()?;
            let (left, left_width) = self.register_range_id()?;
            let (right, right_width) = self.register_range_id()?;
            let ty = self.formatter.context().register_type(left)?;

            // require every physical range to carry one complete logical value
            if result_width != ty.word_count()
                || left_width != ty.word_count()
                || right_width != ty.word_count()
            {
                return Err(FormatError::SyntaxError {
                    message: "select width does not match its value type",
                });
            }

            (result, condition, left, right, ty)
        };

        // write the typed selection
        self.write_result(result, ty)?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("select"), space()]
        )?;
        self.write_register(condition)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(right)
    }

    /// Format exact equality between matching one-register values.
    fn format_equal(&mut self) -> FormatResult<()> {
        self.result(ValueType::scalar(Scalar::Boolean))?;
        let left = self.register_id()?;
        let right = self.register_id()?;
        let name = Opcode::EQUAL.name().ok_or(FormatError::SyntaxError {
            message: "equality has no canonical name",
        })?;

        // write both exact value operands
        write!(
            self.formatter,
            [space(), token("="), space(), token(name), space()]
        )?;
        self.write_register(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(right)
    }
}
