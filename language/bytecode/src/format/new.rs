use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{New, NewKind, TypeId};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one `new` operation.
    pub(super) fn format_new(&mut self, operation: New) -> FormatResult<()> {
        // decode the physical result and canonical operation qualifiers
        let (result, word_count) = if operation.kind == NewKind::Slice {
            self.register_range_id()?
        } else {
            (self.register_id()?, 1)
        };
        let space_name = operation.space.name().ok_or(FormatError::SyntaxError {
            message: "new operation has an invalid space",
        })?;
        let ownership_name = operation.ownership.name().ok_or(FormatError::SyntaxError {
            message: "new operation has an invalid ownership",
        })?;

        // resolve the allocated runtime type and require the result width
        let (type_name, symbol) = self.symbol_with_target()?;
        let ty = TypeId(symbol.index);
        let result_type = operation.result_type(ty);
        if word_count != result_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "new result width does not match its type",
            });
        }

        // write the result and canonical operation name
        self.write_result(result, result_type)?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("new.")]
        )?;
        self.write_text(space_name)?;
        self.write_token(".")?;
        self.write_text(ownership_name)?;
        if operation.kind == NewKind::Slice {
            self.write_token(".slice")?;
        }
        self.write_token(".")?;
        self.write_text(operation.initialization.name())?;
        if operation.is_fallible {
            self.write_token(".try")?;
        }
        write!(self.formatter, [space()])?;
        self.write_text(&type_name)?;

        // write the variable slice length
        if operation.kind == NewKind::Slice {
            let length = self.register_id()?;
            write!(self.formatter, [token(","), space()])?;
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
