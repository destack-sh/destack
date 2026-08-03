use destack_fir::format::{FormatError, FormatResult};

use crate::{
    BooleanOperation, CastOperation, IntegerOperation, Opcode, Operand, RegisterSpan, Scalar,
    ValueType,
};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one constant instruction.
    pub(super) fn format_named_constant(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::CONSTANT_TYPE => self.format_type_constant(),
            Opcode::CONSTANT_INT128 | Opcode::CONSTANT_UINT128 => self.format_wide_constant(opcode),
            Opcode::CONSTANT_NULL | Opcode::CONSTANT_UNDEFINED => self.format_nullish(opcode),
            Opcode::CONSTANT_ZEROED => self.format_zeroed(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid constant opcode",
            }),
        }
    }

    /// Format one linked runtime type identity.
    fn format_type_constant(&mut self) -> FormatResult<()> {
        self.write_opcode("constant")?;
        self.result()?;
        let symbol = self.relocation_text()?;

        // write the symbolic type and its representation
        self.write_comma()?;
        self.write_text(&symbol)?;
        self.write_representation(ValueType::type_id())
    }

    /// Format one signed or unsigned 128 bit constant.
    fn format_wide_constant(&mut self, opcode: Opcode) -> FormatResult<()> {
        let ty = if opcode == Opcode::CONSTANT_INT128 {
            ValueType::int128()
        } else {
            ValueType::uint128()
        };
        self.write_opcode("constant")?;
        self.result_span()?;

        // preserve the declared signedness in source text
        let bits = self.u128()?;
        let value = if opcode == Opcode::CONSTANT_INT128 {
            (bits as i128).to_string()
        } else {
            bits.to_string()
        };
        self.write_comma()?;
        self.write_text(&value)?;
        self.write_representation(ty)
    }

    /// Format one nullish reference-like constant.
    fn format_nullish(&mut self, opcode: Opcode) -> FormatResult<()> {
        let literal = if opcode == Opcode::CONSTANT_NULL {
            "null"
        } else {
            "undefined"
        };
        self.write_opcode("constant")?;
        self.result_span()?;

        self.write_comma()?;
        self.write_token(literal)
    }

    /// Format one zero-initialized storage value.
    fn format_zeroed(&mut self) -> FormatResult<()> {
        self.write_opcode("constant")?;
        self.result_span()?;

        self.write_comma()?;
        self.write_token("zeroed")
    }

    /// Format one scalar constant.
    pub(super) fn format_constant(&mut self, scalar: Scalar) -> FormatResult<()> {
        self.write_opcode("constant")?;
        self.result()?;
        let bits = self.u64()?;
        let literal = scalar.literal(bits);

        // write the canonical literal and physical representation
        self.write_comma()?;
        self.write_text(&literal)?;
        self.write_scalar_representation(scalar)
    }

    /// Format one boolean operation.
    pub(super) fn format_boolean(&mut self, operation: BooleanOperation) -> FormatResult<()> {
        let name = format!("boolean.{}", operation.name());

        self.format_scalar_operation(&name, None)
    }

    /// Format one regular scalar operation.
    pub(super) fn format_scalar(&mut self, operation: &str, scalar: Scalar) -> FormatResult<()> {
        let family = if scalar.is_float() { "float" } else { "int" };
        let name = format!("{family}.{operation}");

        self.format_scalar_operation(&name, Some(ValueType::scalar(scalar)))
    }

    /// Format one 128 bit integer operation.
    pub(super) fn format_integer128(
        &mut self,
        operation: IntegerOperation,
        is_signed: bool,
    ) -> FormatResult<()> {
        let ty = if is_signed {
            ValueType::int128()
        } else {
            ValueType::uint128()
        };
        let name = format!("int.{}", operation.name());

        self.format_scalar_operation(&name, Some(ty))
    }

    /// Format one scalar cast.
    pub(super) fn format_cast(
        &mut self,
        operation: CastOperation,
        source: ValueType,
        target: ValueType,
    ) -> FormatResult<()> {
        let operation = operation
            .name(source, target)
            .ok_or(FormatError::SyntaxError {
                message: "cast has no canonical name",
            })?;
        let name = format!("cast.{operation}");
        self.write_opcode(&name)?;
        self.result()?;
        let input = self.register_id()?;

        // write the input and exact conversion
        self.write_comma()?;
        self.write_register(input)?;
        self.write_conversion(source, target)
    }

    /// Format one scalar operation from its exact operand layout.
    fn format_scalar_operation(
        &mut self,
        name: &str,
        representation: Option<ValueType>,
    ) -> FormatResult<()> {
        let layout = self
            .instruction
            .opcode()
            .layout()
            .ok_or(FormatError::SyntaxError {
                message: "scalar opcode has no operand layout",
            })?;
        self.write_opcode(name)?;

        // write operands in their exact encoded order
        for (index, operand) in layout.operands().iter().copied().enumerate() {
            if index > 0 {
                self.write_comma()?;
            }
            match operand {
                Operand::Result | Operand::Register => {
                    let register = self.register_id()?;
                    self.write_register(register)?;
                }
                Operand::ResultRange | Operand::RegisterSpan => {
                    let (register, word_count) = self.register_span_id()?;
                    self.write_span(RegisterSpan::new(register, word_count))?;
                }
                _ => {
                    return Err(FormatError::SyntaxError {
                        message: "scalar operation has a specialized operand",
                    });
                }
            }
        }

        // retain only representation information not implied by the family
        if let Some(representation) = representation {
            self.write_representation(representation)?;
        }

        Ok(())
    }
}

impl Scalar {
    /// Format one scalar immediate from its exact register bits.
    fn literal(self, bits: u64) -> String {
        match self {
            Self::Boolean => (bits != 0).to_string(),
            Self::Int8 => (bits as i8).to_string(),
            Self::Uint8 => (bits as u8).to_string(),
            Self::Int16 => (bits as i16).to_string(),
            Self::Uint16 => (bits as u16).to_string(),
            Self::Int32 => (bits as i32).to_string(),
            Self::Uint32 => (bits as u32).to_string(),
            Self::Int64 => (bits as i64).to_string(),
            Self::Uint64 => bits.to_string(),
            Self::Float16 | Self::Bfloat16 | Self::Float32 | Self::Float64 => {
                let Some(value) = self.float(bits) else {
                    unreachable!("floating point scalars have one concrete format");
                };

                Self::float_literal(value, self.encode(bits))
            }
        }
    }

    /// Format one floating point value accepted by the assembler.
    fn float_literal(value: f64, bits: u64) -> String {
        if value.is_nan() {
            format!("bits(0x{bits:x})")
        } else if value == f64::INFINITY {
            "Infinity".to_string()
        } else if value == f64::NEG_INFINITY {
            "-Infinity".to_string()
        } else {
            value.to_string()
        }
    }
}
