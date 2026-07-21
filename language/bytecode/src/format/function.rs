use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BytecodeFormatter, CodeOffset, CodeRange, Function, FunctionId, FunctionType, Linkage, Opcode,
    SymbolTag, ValueType,
};

use super::instruction::InstructionFormatter;

impl FunctionType {
    /// Format this function type's parameter and result types.
    pub(crate) fn format_types<'a>(
        &self,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let types = formatter.context().object.value_types();
        format_type_list(self.parameters(types), formatter)?;
        write!(formatter, [space(), token("=>"), space()])?;
        format_results(self.results(types), formatter)
    }
}

impl Function {
    /// Format this function declaration or definition.
    pub(crate) fn format_at<'a>(
        &self,
        id: FunctionId,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let context = formatter.context();
        let name = context.string(self.name)?.to_string();
        let function_type = context
            .object
            .function_type(self.function_type)
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "function references a missing function type",
            })?;

        // write the function header
        self.linkage.format(formatter)?;
        write!(formatter, [token("function"), space(), copied_text(&name)])?;
        self.format_parameters(&function_type, self.linkage != Linkage::EXTERNAL, formatter)?;
        self.format_resume(formatter)?;
        write!(formatter, [token(":"), space()])?;
        let types = formatter.context().object.value_types();
        format_results(function_type.results(types), formatter)?;

        // external functions end at the declaration
        let Some(code) = self.code() else {
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
        function_type: &FunctionType,
        is_defined: bool,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let types = formatter.context().object.value_types();
        let parameters = function_type.parameters(types);
        let values = format_with(|formatter| {
            let mut register = 0u16;
            let environment = self.environment.get();

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

    /// Format this function's resume values when it can suspend.
    fn format_resume<'a>(&self, formatter: &mut BytecodeFormatter<'a, '_>) -> FormatResult<()> {
        let types = formatter.context().object.value_types();
        let resume = self.resume_types(types);
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
        let slots = self.frame_slots(object.frame_slots());

        // write logical frame slots
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
                        copied_text(&ty),
                        hard_line_break()
                    ]
                )?;
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
            Opcode::FUNCTION_POINTER => self.format_function_pointer(),
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
        let function = self
            .formatter
            .context()
            .object
            .function(FunctionId(symbol.index))
            .ok_or(FormatError::SyntaxError {
                message: "function address references a missing function",
            })?;
        self.write_result(result, ValueType::function_pointer(function.function_type))?;
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
        let function = self
            .formatter
            .context()
            .object
            .function(FunctionId(symbol.index))
            .ok_or(FormatError::SyntaxError {
                message: "function binding references a missing function",
            })?;
        let ty = ValueType::function(function.function_type);
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

    /// Format one bare pointer projection from a function value.
    fn format_function_pointer(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let (function, word_count) = self.register_range_id()?;
        let function_type = self
            .formatter
            .context()
            .register_type(function)?
            .function_type()
            .ok_or(FormatError::SyntaxError {
                message: "function.pointer reads a non-function value",
            })?;
        if word_count != ValueType::function(function_type).word_count() {
            return Err(FormatError::SyntaxError {
                message: "function value has an invalid register width",
            });
        }
        self.write_result(result, ValueType::function_pointer(function_type))?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("function.pointer"),
                space()
            ]
        )?;
        self.write_register(function)
    }

    /// Format one captured environment projection from a function value.
    fn format_function_environment(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let ty = self.value_type()?;
        let (function, word_count) = self.register_range_id()?;
        let function_type = self.formatter.context().register_type(function)?;
        if !function_type.is_function() || word_count != function_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "function.environment reads an invalid function value",
            });
        }
        self.write_result(result, ty)?;
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
