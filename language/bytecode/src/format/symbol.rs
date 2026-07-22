use destack_fir::format::{FormatError, FormatResult};

use crate::{
    BytecodeFormatContext, CodeOffset, ConstantId, DynamicRelocation, FunctionId, GlobalId, Symbol,
    SymbolTag, TypeId,
};

impl<'a> BytecodeFormatContext<'a> {
    /// Return one required relocation target in the active function.
    pub(super) fn relocation(&self, offset: CodeOffset) -> FormatResult<Symbol> {
        let byte_offset = self.absolute_code_offset(offset)?;

        self.relocations
            .get(&byte_offset)
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "bytecode object is missing an instruction relocation",
            })
    }

    /// Return one required dynamic relocation in the active function.
    pub(super) fn dynamic_relocation(&self, offset: CodeOffset) -> FormatResult<DynamicRelocation> {
        let byte_offset = self.absolute_code_offset(offset)?;

        self.dynamic_relocations
            .get(&byte_offset)
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "bytecode object is missing a dynamic relocation",
            })
    }

    /// Return the stable name selected by one relocation target.
    pub(super) fn relocation_name(&self, target: Symbol) -> FormatResult<&'a str> {
        let name = if target.tag == SymbolTag::TYPE {
            return self.type_name(TypeId(target.index));
        } else if target.tag == SymbolTag::GLOBAL {
            self.object
                .global(GlobalId(target.index))
                .map(|global| global.name)
        } else if target.tag == SymbolTag::FUNCTION {
            self.object
                .function(FunctionId(target.index))
                .map(|function| function.name)
        } else if target.tag == SymbolTag::CONSTANT {
            self.object
                .constant(ConstantId(target.index))
                .and_then(|constant| constant.name.get())
        } else {
            None
        };
        let name = name.ok_or(FormatError::SyntaxError {
            message: "bytecode relocation has no printable symbol",
        })?;

        self.string(name)
    }

    /// Return one function-relative offset in the complete code section.
    fn absolute_code_offset(&self, offset: CodeOffset) -> FormatResult<u32> {
        let function = self.function.ok_or(FormatError::SyntaxError {
            message: "instruction formatted outside a function",
        })?;
        let code = self
            .object
            .function(function)
            .and_then(|function| function.body.code())
            .ok_or(FormatError::SyntaxError {
                message: "instruction formatted outside a function definition",
            })?;

        Ok(code.byte_offset + offset.0)
    }
}
