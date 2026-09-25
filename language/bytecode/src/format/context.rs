use tspp_fir::format::{FormatError, FormatResult};

use crate::{Opcode, RegisterId, RegisterSpan, RelocationTag};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one execution context operation.
    pub(super) fn format_context(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::CONTEXT_CURRENT => self.format_context_current(),
            Opcode::CONTEXT_REPLACE => self.format_context_replace(),
            Opcode::CONTEXT_BIND => self.format_context_bind(),
            Opcode::CONTEXT_GET => self.format_context_get(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid context opcode",
            }),
        }
    }

    /// Format one current context load.
    fn format_context_current(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;

        self.write_opcode("context.current")?;
        self.write_register(result)
    }

    /// Format one current context replacement.
    fn format_context_replace(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let context = self.register_id()?;

        self.write_opcode("context.replace")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(context)
    }

    /// Format one context extension.
    fn format_context_bind(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let context = self.register_id()?;
        let variable = self.register_id()?;
        let (value, word_count) = self.register_span_id()?;
        let (relocation, allocation) = self.relocation()?;
        if relocation.tag != RelocationTag::ALLOCATION {
            return Err(FormatError::SyntaxError {
                message: "context bind does not reference an allocation site",
            });
        }
        let value_offset = self.u32()?;

        self.write_opcode("context.bind")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_context_value(context, variable, RegisterSpan::new(value, word_count))?;
        self.write_comma()?;
        self.write_text(&format!("a{allocation}"))?;
        self.write_comma()?;
        self.write_text(&value_offset.to_string())
    }

    /// Format one context value lookup.
    fn format_context_get(&mut self) -> FormatResult<()> {
        let (result, result_count) = self.register_span_id()?;
        let context = self.register_id()?;
        let variable = self.register_id()?;
        let (default, default_count) = self.register_span_id()?;
        let value_offset = self.u32()?;

        self.write_opcode("context.get")?;
        self.write_span(RegisterSpan::new(result, result_count))?;
        self.write_comma()?;
        self.write_context_value(context, variable, RegisterSpan::new(default, default_count))?;
        self.write_comma()?;
        self.write_text(&value_offset.to_string())
    }

    /// Format one context, variable, and value operand group.
    fn write_context_value(
        &mut self,
        context: RegisterId,
        variable: RegisterId,
        value: RegisterSpan,
    ) -> FormatResult<()> {
        self.write_register(context)?;
        self.write_comma()?;
        self.write_register(variable)?;
        self.write_comma()?;
        self.write_span(value)
    }
}
