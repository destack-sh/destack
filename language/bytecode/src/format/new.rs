use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{New, NewKind, RegisterSpan, RelocationTag};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one `new` operation.
    pub(super) fn format_new(&mut self, operation: New) -> FormatResult<()> {
        let result = if operation.kind == NewKind::Slice {
            let (start, word_count) = self.register_span_id()?;

            RegisterSpan::new(start, word_count)
        } else {
            RegisterSpan::new(self.register_id()?, 1)
        };

        // decode the direct allocation site identity
        let (allocation, index) = self.relocation()?;
        if allocation.tag != RelocationTag::ALLOCATION {
            return Err(FormatError::SyntaxError {
                message: "new operation does not reference an allocation site",
            });
        }
        let allocation = format!("a{index}");

        // write the canonical operation and allocation site
        let mut name = "new".to_string();
        if operation.kind == NewKind::Slice {
            name.push_str(".slice");
        }
        name.push('.');
        name.push_str(operation.initialization.name());
        if operation.is_fallible {
            name.push_str(".try");
        }
        self.write_opcode(&name)?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_text(&allocation)?;

        // write the variable slice length
        if operation.kind == NewKind::Slice {
            let length = self.register_id()?;
            self.write_comma()?;
            self.write_register(length)?;
        }

        // write explicit success and failure edges
        if operation.is_fallible {
            let success = self.branch()?;
            let failure = self.branch()?;
            write!(self.formatter, [space(), token("=>"), space()])?;
            self.write_label(success)?;
            write!(self.formatter, [space(), token("|"), space()])?;
            self.write_label(failure)?;
        }

        Ok(())
    }
}
