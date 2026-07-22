use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BytecodeFormatter, CodeOffset, CodeRange, Function, FunctionId, Linkage, Opcode, SymbolTag,
    ValueType,
};

use super::instruction::InstructionFormatter;

impl Function {
    /// Format this function declaration or definition.
    pub(crate) fn format_at<'a>(
        &self,
        id: FunctionId,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let context = formatter.context();
        let name = context.string(self.name)?.to_string();
        let parameters = self.body.parameters(context.object.value_types()).to_vec();
        let results = self.body.results(context.object.value_types()).to_vec();

        // write the function header
        self.linkage.format(formatter)?;
        write!(formatter, [token("function"), space(), copied_text(&name)])?;
        self.format_parameters(&parameters, self.linkage != Linkage::EXTERNAL, formatter)?;
        self.format_resume(formatter)?;
        write!(formatter, [token(":"), space()])?;
        format_results(&results, formatter)?;

        // external functions end at the declaration
        let Some(code) = self.body.code() else {
            return Ok(());
        };

        // format the logical frame and encoded body
        write!(formatter, [space(), token("{"), hard_line_break()])?;
        formatter.context_mut().begin_function(id, self, code)?;
        self.format_body(code, formatter)?;
        formatter.context_mut().end_function();
        write!(formatter, [token("}")])
    }

    /// Format this function's logical parameters and physical registers.
    fn format_parameters<'a>(
        &self,
        parameters: &[ValueType],
        is_defined: bool,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let values = format_with(|formatter| {
            let mut register = 0u16;
            let environment = self.body.environment.get();

            // write the hidden callable environment before regular parameters
            if is_defined && let Some(ty) = environment {
                write!(formatter, [token("environment"), space()])?;
                format_register(register, formatter)?;
                write!(formatter, [token(":"), space(), ty])?;
                register += ty.word_count();
            }

            // write logical parameters over their physical register words
            for ty in parameters {
                if register > 0 {
                    write!(formatter, [token(","), soft_line_break_or_space()])?;
                }
                if is_defined {
                    format_register(register, formatter)?;
                    write!(formatter, [token(":"), space()])?;
                }
                write!(formatter, [ty])?;
                register += ty.word_count();
            }
            if !parameters.is_empty() {
                write!(formatter, [if_group_breaks(&token(","))])?;
            }

            Ok(())
        });
        let parameters = format_with(|formatter| {
            write!(
                formatter,
                [token("("), soft_block_indent(&values), token(")")]
            )
        });

        write!(formatter, [group(&parameters)])
    }

    /// Format this function's resume parameters when it can suspend.
    fn format_resume<'a>(&self, formatter: &mut BytecodeFormatter<'a, '_>) -> FormatResult<()> {
        let types = formatter.context().object.value_types();
        let resume = self.body.resume_parameters(types);
        if resume.is_empty() {
            return Ok(());
        }

        write!(formatter, [space(), token("resume")])?;
        format_type_list(resume, formatter)
    }

    /// Format this function's frame slots, labels, and instructions.
    fn format_body<'a>(
        &self,
        code: CodeRange,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let object = formatter.context().object;
        let slots = self.body.frame_slots(object.frame_slots());

        // write frame slots
        let frame = format_with(|formatter: &mut BytecodeFormatter<'a, '_>| {
            for (index, slot) in slots.iter().enumerate() {
                let ty = formatter.context().type_name(slot.ty)?.to_string();
                let name = format!("s{index}");
                write!(
                    formatter,
                    [
                        token("slot"),
                        space(),
                        copied_text(&name),
                        token(":"),
                        space(),
                        copied_text(&ty)
                    ]
                )?;
                if let Some(registers) = slot.registers() {
                    write!(formatter, [space(), token("="), space()])?;
                    format_register(registers.start.0, formatter)?;
                    let word_count = registers.word_count.to_string();
                    write!(
                        formatter,
                        [token("["), copied_text(&word_count), token("]")]
                    )?;
                }
                write!(formatter, [hard_line_break()])?;
            }

            Ok(())
        });
        write!(formatter, [block_indent(&frame)])?;
        if !slots.is_empty() {
            write!(formatter, [empty_line()])?;
        }

        // write instruction labels and bodies
        let mut offset = CodeOffset(0);
        for instruction in code.instructions(object.code()) {
            let instruction = instruction.map_err(|_| FormatError::SyntaxError {
                message: "function contains an invalid instruction",
            })?;
            if let Some(label) = formatter.context().label(offset) {
                if offset.0 > 0 {
                    write!(formatter, [empty_line()])?;
                }
                let label = label.to_string();
                write!(
                    formatter,
                    [copied_text(&label), token(":"), hard_line_break()]
                )?;
            }
            write!(
                formatter,
                [block_indent(&format_with(|formatter| {
                    instruction.format_at(offset, formatter)
                }))]
            )?;
            write!(formatter, [hard_line_break()])?;
            offset.0 += instruction.byte_len() as u32;
        }

        Ok(())
    }
}

impl Linkage {
    /// Format this linkage prefix.
    pub(super) fn format<'a>(self, formatter: &mut BytecodeFormatter<'a, '_>) -> FormatResult<()> {
        if self == Self::EXTERNAL {
            write!(formatter, [token("external"), space()])?;
        } else if self == Self::EXPORT {
            write!(formatter, [token("export"), space()])?;
        }

        Ok(())
    }
}

/// Format one parenthesized logical type list.
fn format_type_list<'a>(
    types: &[ValueType],
    formatter: &mut BytecodeFormatter<'a, '_>,
) -> FormatResult<()> {
    let values = format_with(|formatter| {
        for (index, ty) in types.iter().enumerate() {
            if index > 0 {
                write!(formatter, [token(","), soft_line_break_or_space()])?;
            }
            write!(formatter, [ty])?;
        }

        Ok(())
    });
    let types = format_with(|formatter| {
        write!(
            formatter,
            [token("("), soft_block_indent(&values), token(")")]
        )
    });

    write!(formatter, [group(&types)])
}

/// Format one callable result type list.
fn format_results<'a>(
    results: &[ValueType],
    formatter: &mut BytecodeFormatter<'a, '_>,
) -> FormatResult<()> {
    if results.is_empty() {
        write!(formatter, [token("void")])
    } else if results.len() == 1 {
        write!(formatter, [&results[0]])
    } else {
        format_type_list(results, formatter)
    }
}

/// Format the head register of one logical value.
fn format_register<'a>(
    register: u16,
    formatter: &mut BytecodeFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(formatter, [copied_text(&format!("r{register}"))])
}

impl InstructionFormatter<'_, '_, '_> {
    /// Format one function value operation.
    pub(super) fn format_function_value(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::FUNCTION_ADDRESS => self.format_function_address(),
            Opcode::FUNCTION_BIND => self.format_function_bind(),
            Opcode::FUNCTION_ENVIRONMENT => self.format_function_environment(),
            Opcode::FUNCTION_ENVIRONMENT_CURRENT => self.format_current_environment(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid function value opcode",
            }),
        }
    }

    /// Format one linked function address.
    fn format_function_address(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let (name, symbol) = self.symbol_with_target()?;
        if symbol.tag != SymbolTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "function address does not reference a function",
            });
        }
        self.write_result(result, ValueType::function_pointer())?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("function.address"),
                space()
            ]
        )?;
        self.write_text(&name)
    }

    /// Format one function and captured environment binding.
    fn format_function_bind(&mut self) -> FormatResult<()> {
        let (result, word_count) = self.register_range_id()?;
        let (name, symbol) = self.symbol_with_target()?;
        if symbol.tag != SymbolTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "function binding does not reference a function",
            });
        }
        let ty = ValueType::function();
        if word_count != ty.word_count() {
            return Err(FormatError::SyntaxError {
                message: "function binding result has an invalid register width",
            });
        }
        let environment = self.register_id()?;
        self.write_result(result, ty)?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("function.bind"),
                space()
            ]
        )?;
        self.write_text(&name)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(environment)
    }

    /// Format one captured environment projection from a function value.
    fn format_function_environment(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let result_type = self.value_type()?;
        let (function, word_count) = self.register_range_id()?;
        let function_value = self.formatter.context().register_type(function)?;
        if !function_value.is_function() || word_count != function_value.word_count() {
            return Err(FormatError::SyntaxError {
                message: "function.environment reads an invalid function value",
            });
        }
        self.write_result(result, result_type)?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("function.environment"),
                space()
            ]
        )?;
        self.write_register(function)
    }

    /// Format one current captured environment access.
    fn format_current_environment(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let ty = self
            .formatter
            .context()
            .active_function()?
            .body
            .environment
            .get()
            .ok_or(FormatError::SyntaxError {
                message: "function has no current environment",
            })?;
        self.write_result(result, ty)?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("function.environment.current")
            ]
        )
    }
}
