use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, ReferenceType, RegisterSpan, RelocationTag};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one reference lifetime or storage operation.
    pub(super) fn format_reference(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::FREE => self.format_reference_lifetime(opcode),
            Opcode::DROP => self.format_drop(),
            Opcode::PIN | Opcode::UNPIN => self.format_reference_lifetime(opcode),
            Opcode::BARRIER => self.format_barrier(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid reference opcode",
            }),
        }
    }

    /// Format one managed pin transition or unique release.
    fn format_reference_lifetime(&mut self, opcode: Opcode) -> FormatResult<()> {
        // decode the affected reference
        let value = self.register_id()?;
        let reference = self.reference()?;
        let name = self.opcode_name(opcode)?;

        // write the lifetime operation
        self.write_reference_opcode(name, reference)?;
        self.write_register(value)?;

        Ok(())
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
        self.write_reference_opcode("barrier", reference)?;
        self.write_register(object)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(offset)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(byte_len)
    }

    /// Write one operation selected by reference space and ownership.
    pub(super) fn write_reference_opcode(
        &mut self,
        operation: &str,
        reference: ReferenceType,
    ) -> FormatResult<()> {
        let space = reference
            .storage()
            .heap_space()
            .and_then(|space| space.name())
            .ok_or(FormatError::SyntaxError {
                message: "reference operation requires heap storage",
            })?;
        let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
            message: "reference operation has an invalid ownership",
        })?;
        let name = format!("{operation}.{space}.{kind}");

        self.write_opcode(&name)
    }
}
