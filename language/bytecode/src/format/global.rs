use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{BytecodeFormatter, FunctionId, Global, GlobalLocation, Linkage, SymbolTag};

impl Global {
    /// Format this global declaration or definition.
    pub(crate) fn format<'a>(&self, formatter: &mut BytecodeFormatter<'a, '_>) -> FormatResult<()> {
        let context = formatter.context();
        let name = context.string(self.name)?.to_string();
        let ty = context.type_name(self.ty)?.to_string();

        // write declaration modifiers
        self.linkage.format(formatter)?;
        if self.location == GlobalLocation::LOCAL_STATIC {
            write!(formatter, [token("local"), space()])?;
        } else if self.location == GlobalLocation::SHARED_STATIC {
            write!(formatter, [token("shared"), space()])?;
        }
        if !self.is_mutable() {
            write!(formatter, [token("readonly"), space()])?;
        }

        // write the global declaration
        write!(
            formatter,
            [
                token("global"),
                space(),
                copied_text(&name),
                token(":"),
                space(),
                copied_text(&ty)
            ]
        )?;
        if self.linkage != Linkage::EXTERNAL {
            write!(formatter, [space(), token("="), space()])?;
            self.format_initializer(formatter)?;
        }

        Ok(())
    }

    /// Format this global's zero, constant, or function initializer.
    fn format_initializer<'a>(
        &self,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let Some(constant_id) = self.initializer() else {
            return write!(formatter, [token("zero")]);
        };
        let context = formatter.context();
        let constant = context
            .object
            .constant(constant_id)
            .ok_or(FormatError::SyntaxError {
                message: "global references a missing constant",
            })?;

        // use a named constant directly
        if let Some(name) = constant.name.get() {
            let name = context.string(name)?;

            return write!(formatter, [token("constant"), space(), copied_text(name)]);
        }

        // resolve anonymous function address constants
        let relocation = context
            .object
            .constant_relocations()
            .iter()
            .find(|relocation| {
                let byte_offset = relocation.byte_offset;
                byte_offset >= constant.value.bytes.start
                    && byte_offset < constant.value.bytes.end()
            })
            .ok_or(FormatError::SyntaxError {
                message: "anonymous global constant has no relocation",
            })?;
        if relocation.symbol.tag != SymbolTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "anonymous global constant does not reference a function",
            });
        }
        let function = context
            .object
            .function(FunctionId(relocation.symbol.index))
            .ok_or(FormatError::SyntaxError {
                message: "global references a missing function",
            })?;
        let name = context.string(function.name)?;

        write!(formatter, [token("function"), space(), copied_text(name)])
    }
}
