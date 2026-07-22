use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, Scalar, ValueTag, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one slice operation.
    pub(super) fn format_slice(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::SLICE_VIEW => self.format_slice_view(),
            Opcode::SLICE_LENGTH => self.format_slice_length(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid slice opcode",
            }),
        }
    }

    /// Format one contiguous slice subview.
    fn format_slice_view(&mut self) -> FormatResult<()> {
        // decode result and source slice ranges
        let (result, result_word_count) = self.register_range_id()?;
        let (source, source_word_count) = self.register_range_id()?;
        let element = self.u32()?;
        let ty = self.formatter.context().register_type(source)?;

        // require one complete initialized slice input
        if ty.tag() != ValueTag::SLICE
            || result_word_count != ty.word_count()
            || source_word_count != ty.word_count()
            || ty.slice_element().is_none_or(|ty| ty.0 != element)
        {
            return Err(FormatError::SyntaxError {
                message: "slice.view has an invalid slice register range",
            });
        }

        // decode the dynamic subrange
        let start = self.register_id()?;
        let length = self.register_id()?;

        // write the complete slice operation
        self.write_result(result, ty)?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("slice.view"), space()]
        )?;
        self.write_register(source)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(start)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(length)
    }

    /// Format one slice element count.
    fn format_slice_length(&mut self) -> FormatResult<()> {
        // decode one complete slice input
        self.result(ValueType::scalar(Scalar::Uint64))?;
        let (slice, word_count) = self.register_range_id()?;
        let ty = self.formatter.context().register_type(slice)?;

        // require one complete initialized slice input
        if ty.tag() != ValueTag::SLICE || word_count != ty.word_count() {
            return Err(FormatError::SyntaxError {
                message: "slice.length has an invalid slice register range",
            });
        }

        // write the length projection
        write!(
            self.formatter,
            [space(), token("="), space(), token("slice.length"), space()]
        )?;
        self.write_register(slice)
    }
}
