use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one dynamic value operation.
    pub(super) fn format_dynamic(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::DYNAMIC_BIND => self.format_dynamic_bind(),
            Opcode::DYNAMIC_PAYLOAD => self.format_dynamic_payload(),
            Opcode::DYNAMIC_TYPE => self.format_dynamic_type(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid dynamic opcode",
            }),
        }
    }

    /// Format one dynamic value construction.
    fn format_dynamic_bind(&mut self) -> FormatResult<()> {
        // decode result, payload, and dispatch table
        let (result, word_count) = self.register_range_id()?;
        let value = self.register_id()?;
        let relocation = self.dynamic_relocation()?;
        let concrete = self
            .formatter
            .context()
            .type_name(relocation.concrete)?
            .to_string();
        let ty = self.formatter.context().register_type(result)?;

        // match the encoded result and dynamic dispatch relocation
        if word_count != ty.word_count()
            || ty.constraint() != Some(relocation.constraint)
            || !ty.is_dynamic()
        {
            return Err(FormatError::SyntaxError {
                message: "dynamic binding result does not match its relocation",
            });
        }

        // write the dynamic construction
        self.write_result(result, ty)?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("dynamic.bind"), space()]
        )?;
        self.write_register(value)?;
        write!(self.formatter, [token(":"), space()])?;
        self.write_text(&concrete)
    }

    /// Format one erased dynamic payload access.
    fn format_dynamic_payload(&mut self) -> FormatResult<()> {
        // decode one complete dynamic value
        let result = self.register_id()?;
        let (dynamic, word_count) = self.register_range_id()?;
        let dynamic_type = self.formatter.context().register_type(dynamic)?;
        let reference = dynamic_type
            .dynamic_reference()
            .ok_or(FormatError::SyntaxError {
                message: "dynamic.payload reads a non-dynamic value",
            })?;
        let result_type = ValueType::reference(reference.kind(), reference.space());

        // require one complete dynamic input
        if word_count != dynamic_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "dynamic.payload reads an invalid dynamic value",
            });
        }

        // write the erased payload projection
        self.write_result(result, result_type)?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("dynamic.payload"),
                space()
            ]
        )?;
        self.write_register(dynamic)
    }

    /// Format one dynamic runtime type access.
    fn format_dynamic_type(&mut self) -> FormatResult<()> {
        // decode one complete dynamic value
        self.result(ValueType::type_id())?;
        let (dynamic, word_count) = self.register_range_id()?;

        // require one complete dynamic input
        if word_count != 2 {
            return Err(FormatError::SyntaxError {
                message: "dynamic.type reads an invalid dynamic value",
            });
        }

        // write the runtime type projection
        write!(
            self.formatter,
            [space(), token("="), space(), token("dynamic.type"), space()]
        )?;
        self.write_register(dynamic)
    }
}
