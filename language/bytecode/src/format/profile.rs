use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::Opcode;

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one explicit profile operation.
    pub(super) fn format_profile(&mut self, opcode: Opcode) -> FormatResult<()> {
        let name = match opcode {
            Opcode::PROFILE_INCREMENT => "profile.increment",
            Opcode::PROFILE_SAMPLE => "profile.sample",
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "invalid profile opcode",
                });
            }
        };
        let instrument = if opcode == Opcode::PROFILE_INCREMENT {
            self.counter()?.0.to_string()
        } else {
            self.sampler()?.0.to_string()
        };
        let instrument_name = if opcode == Opcode::PROFILE_INCREMENT {
            "counter("
        } else {
            "sampler("
        };
        write!(
            self.formatter,
            [token(name), space(), token(instrument_name)]
        )?;
        self.write_text(&instrument)?;
        self.write_token(")")?;

        // append the sampled register when present
        if opcode == Opcode::PROFILE_SAMPLE {
            let value = self.register_id()?;
            write!(self.formatter, [token(","), space()])?;
            self.write_register(value)?;
        }

        Ok(())
    }
}
