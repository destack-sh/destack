use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BytecodeFormatter, CodeOffset, CodeRange, Function, FunctionId, Opcode, RegisterSpan,
    RelocationTag,
};

use super::instruction::InstructionFormatter;

impl Function {
    /// Format this physical function declaration or definition.
    pub(crate) fn format_at<'a>(
        &self,
        id: FunctionId,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let name = formatter.context().function_name(id)?.to_string();

        // write TS-compatible coroutine modifiers
        if self.coroutine.is_async() {
            write!(formatter, [token("async"), space()])?;
        }
        write!(formatter, [token("function")])?;
        if self.coroutine.is_generator() {
            write!(formatter, [token("*")])?;
        }
        write!(formatter, [space(), copied_text(&name)])?;

        // write physical entry ranges and semantic type identities
        let parameters = self
            .parameters(formatter.context().object.parameters())
            .to_vec();
        write!(formatter, [token("(")])?;
        for (index, parameter) in parameters.into_iter().enumerate() {
            if index > 0 {
                write!(formatter, [token(","), space()])?;
            }
            let registers = formatter.context().register_text(parameter.registers);
            let ty = formatter.context().type_text(parameter.ty);
            write!(
                formatter,
                [
                    copied_text(&registers),
                    token(":"),
                    space(),
                    copied_text(&ty)
                ]
            )?;
        }
        let result = formatter.context().type_text(self.result);
        write!(
            formatter,
            [token(")"), token(":"), space(), copied_text(&result)]
        )?;

        // declarations carry no executable state
        let Some(code) = self.code() else {
            return Ok(());
        };

        // open the physical function body
        write!(formatter, [space(), token("{"), hard_line_break()])?;
        formatter.context_mut().begin_function(id, code)?;

        self.format_body(code, formatter)?;

        formatter.context_mut().end_function()?;
        write!(formatter, [token("}")])
    }

    /// Format this function's labels and instructions.
    fn format_body<'a>(
        &self,
        code: CodeRange,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let object = formatter.context().object;
        let mut offset = CodeOffset(0);

        // write instruction labels and bodies
        for instruction in code.instructions(object.code()) {
            let instruction = instruction.map_err(|_| FormatError::SyntaxError {
                message: "function contains an invalid instruction",
            })?;
            if let Some(label) = formatter.context().label(offset) {
                if offset.0 > 0 {
                    write!(formatter, [empty_line()])?;
                }
                write!(
                    formatter,
                    [
                        copied_text(&label.to_string()),
                        token(":"),
                        hard_line_break()
                    ]
                )?;
            }

            write!(
                formatter,
                [
                    block_indent(&format_with(|formatter| {
                        instruction.format_at(offset, formatter)
                    })),
                    hard_line_break()
                ]
            )?;
            offset.0 += instruction.byte_len() as u32;
        }

        Ok(())
    }
}

impl InstructionFormatter<'_, '_, '_> {
    /// Format one function value operation.
    pub(super) fn format_function_value(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::FUNCTION_ADDRESS => self.format_function_address(),
            Opcode::FUNCTION_BIND => self.format_function_bind(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid function value opcode",
            }),
        }
    }

    /// Format one linked function address.
    fn format_function_address(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let (function, relocation) = self.relocation_with_text()?;
        if relocation.tag != RelocationTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "function address does not reference a function",
            });
        }

        self.write_opcode("function.address")?;
        self.write_result(result)?;
        self.write_comma()?;
        self.write_text(&function)
    }

    /// Format one function and captured environment binding.
    fn format_function_bind(&mut self) -> FormatResult<()> {
        let (start, word_count) = self.register_span_id()?;
        let result = RegisterSpan::new(start, word_count);
        let (function, relocation) = self.relocation_with_text()?;
        if relocation.tag != RelocationTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "function binding does not reference a function",
            });
        }
        let environment = self.register_id()?;

        self.write_opcode("function.bind")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_text(&function)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(environment)
    }
}
