use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, RegisterSpan, RelocationTag, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one reference lifetime or storage operation.
    pub(super) fn format_reference(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::RELEASE => self.format_owner("release"),
            Opcode::FREE => self.format_owner("free"),
            Opcode::DROP => self.format_drop(),
            Opcode::BARRIER => self.format_barrier(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid reference opcode",
            }),
        }
    }

    /// Format one operation on an allocation owner.
    fn format_owner(&mut self, text: &str) -> FormatResult<()> {
        let owner = self.register_id()?;

        self.write_opcode(text)?;
        self.write_register(owner)
    }

    /// Format one explicit value destruction.
    fn format_drop(&mut self) -> FormatResult<()> {
        // decode the value and linked destructor
        let (value, word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(value, word_count);
        let (destructor, relocation) = self.relocation_with_text()?;
        if relocation.tag != RelocationTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "drop does not reference a function",
            });
        }

        // write the destruction
        self.write_token("drop ")?;
        self.write_span(value)?;
        self.write_comma()?;
        self.write_text(&destructor)
    }

    /// Format one managed reference write barrier.
    fn format_barrier(&mut self) -> FormatResult<()> {
        // decode the changed object byte range
        let object = self.register_id()?;
        let reference = self.reference()?;
        let offset = self.register_id()?;
        let byte_len = self.register_id()?;

        // write the barrier range
        self.write_opcode("barrier")?;
        self.write_register(object)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(offset)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(byte_len)?;
        self.write_representation(ValueType::reference(reference.kind(), reference.storage()))
    }
}
