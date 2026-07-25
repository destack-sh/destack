use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, RegisterSpan};

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
        let (result, input) = if opcode == Opcode::MOVE {
            let result = self.register_id()?;
            let input = self.register_id()?;

            (RegisterSpan::new(result, 1), RegisterSpan::new(input, 1))
        } else {
            let (result, result_width) = self.register_span_id()?;
            let (input, input_width) = self.register_span_id()?;

            (
                RegisterSpan::new(result, result_width),
                RegisterSpan::new(input, input_width),
            )
        };

        // write the physical move independent of its width
        self.write_opcode("move")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_span(input)
    }

    /// Format one scalar value selection.
    fn format_select(&mut self, opcode: Opcode) -> FormatResult<()> {
        let (result, condition, left, right) = if opcode == Opcode::SELECT {
            let result = self.register_id()?;
            let condition = self.register_id()?;
            let left = self.register_id()?;
            let right = self.register_id()?;

            (
                RegisterSpan::new(result, 1),
                condition,
                RegisterSpan::new(left, 1),
                RegisterSpan::new(right, 1),
            )
        } else {
            let (result, result_width) = self.register_span_id()?;
            let condition = self.register_id()?;
            let (left, left_width) = self.register_span_id()?;
            let (right, right_width) = self.register_span_id()?;
            (
                RegisterSpan::new(result, result_width),
                condition,
                RegisterSpan::new(left, left_width),
                RegisterSpan::new(right, right_width),
            )
        };

        // write the physical selection
        self.write_opcode("select")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_register(condition)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_span(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_span(right)
    }

    /// Format exact equality between matching one-register values.
    fn format_equal(&mut self) -> FormatResult<()> {
        let name = Opcode::EQUAL.name().ok_or(FormatError::SyntaxError {
            message: "equality has no canonical name",
        })?;
        self.write_opcode(name)?;
        self.result()?;
        let left = self.register_id()?;
        let right = self.register_id()?;

        // write both exact value operands
        self.write_comma()?;
        self.write_register(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(right)
    }
}
