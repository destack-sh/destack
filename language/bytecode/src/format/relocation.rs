use tspp_fir::format::{FormatError, FormatResult};

use crate::{BytecodeFormatContext, CodeOffset, Function, FunctionId, Relocation, RelocationTag};

impl<'a> BytecodeFormatContext<'a> {
    /// Return one required relocation target in the active function.
    pub(super) fn relocation(&self, offset: CodeOffset) -> FormatResult<Relocation> {
        let byte_offset = self.absolute_code_offset(offset)?;

        self.relocations
            .get(&byte_offset)
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "bytecode object is missing an instruction relocation",
            })
    }

    /// Return the canonical text selected by one relocation target.
    pub(super) fn relocation_text(
        &self,
        relocation: Relocation,
        index: u32,
    ) -> FormatResult<String> {
        if relocation.tag == RelocationTag::FUNCTION {
            let function = FunctionId(index);

            return Ok(self.function_name(function)?.to_string());
        }

        let prefix = match relocation.tag {
            RelocationTag::TYPE => 't',
            RelocationTag::LAYOUT => 'l',
            RelocationTag::GLOBAL => 'g',
            RelocationTag::DYNAMIC => 'd',
            RelocationTag::ALLOCATION => 'a',
            RelocationTag::COUNTER => 'c',
            RelocationTag::SAMPLER => 's',
            RelocationTag::FUNCTION => unreachable!("function relocation handled above"),
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "bytecode relocation has an invalid tag",
                });
            }
        };

        Ok(format!("{prefix}{index}"))
    }

    /// Return one function-relative offset in the complete code section.
    fn absolute_code_offset(&self, offset: CodeOffset) -> FormatResult<u32> {
        let function = self.function.ok_or(FormatError::SyntaxError {
            message: "instruction formatted outside a function",
        })?;
        let code = self
            .object
            .function(function)
            .and_then(Function::code)
            .ok_or(FormatError::SyntaxError {
                message: "instruction formatted outside a function definition",
            })?;

        Ok(code.byte_offset + offset.0)
    }
}
