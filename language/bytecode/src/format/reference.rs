use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::Opcode;

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
        self.reference()?;
        let name = self.opcode_name(opcode)?;

        // write the lifetime operation
        self.write_text(name)?;
        self.write_token(" ")?;
        self.write_register(value)
    }

    /// Format one explicit value destruction.
    fn format_drop(&mut self) -> FormatResult<()> {
        // decode the value and concrete type
        let (value, _) = self.register_range_id()?;
        let ty = self.symbol()?;

        // write the destruction
        self.write_token("drop ")?;
        self.write_register(value)?;
        write!(self.formatter, [token(":"), space()])?;
        self.write_text(&ty)
    }

    /// Format one managed reference write barrier.
    fn format_barrier(&mut self) -> FormatResult<()> {
        // decode the changed object byte range
        let object = self.register_id()?;
        self.reference()?;
        let offset = self.register_id()?;
        let byte_len = self.register_id()?;

        // write the barrier range
        write!(self.formatter, [token("barrier"), space()])?;
        self.write_register(object)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(offset)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(byte_len)
    }
}
