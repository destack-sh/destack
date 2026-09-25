use crate::Opcode;
use tspp_fir::format::{FormatError, FormatResult};

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
            format!("c{}", self.counter()?.0)
        } else {
            format!("s{}", self.sampler()?.0)
        };
        self.write_opcode(name)?;
        self.write_text(&instrument)?;

        // append the sampled register when present
        if opcode == Opcode::PROFILE_SAMPLE {
            let value = self.register_id()?;
            self.write_comma()?;
            self.write_register(value)?;
        }

        Ok(())
    }
}
