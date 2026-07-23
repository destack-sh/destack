use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BooleanOperation, CastOperation, FloatOperation, IntegerOperation, Opcode, Operand, RegisterId,
    Scalar, ValueType, Word,
};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one directly named constant instruction.
    pub(super) fn format_named_constant(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::CONSTANT_TYPE => self.format_type_constant(),
            Opcode::CONSTANT_BYTES => self.format_constant_bytes(),
            Opcode::CONSTANT_INT128 | Opcode::CONSTANT_UINT128 => self.format_wide_constant(opcode),
            Opcode::CONSTANT_NULL | Opcode::CONSTANT_UNDEFINED => self.format_nullish(opcode),
            Opcode::CONSTANT_UNINIT | Opcode::CONSTANT_ZEROED => {
                self.format_storage_constant(opcode)
            }
            _ => Err(FormatError::SyntaxError {
                message: "invalid constant opcode",
            }),
        }
    }

    /// Format one linked runtime type identity.
    fn format_type_constant(&mut self) -> FormatResult<()> {
        self.result(ValueType::type_id())?;
        let symbol = self.symbol()?;

        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("constant.type"),
                space()
            ]
        )?;
        self.write_text(&symbol)
    }

    /// Format one immutable byte sequence constant.
    fn format_constant_bytes(&mut self) -> FormatResult<()> {
        let (result, word_count) = self.register_range_id()?;
        if word_count != 2 {
            return Err(FormatError::SyntaxError {
                message: "constant byte result must occupy two register words",
            });
        }

        // write the pointer and byte length results
        self.write_result(result, ValueType::pointer())?;
        write!(self.formatter, [token(","), space()])?;
        self.write_result(RegisterId(result.0 + 1), ValueType::scalar(Scalar::Uint64))?;

        // write the linked constant name
        let constant = self.symbol()?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("constant.bytes"),
                space()
            ]
        )?;
        self.write_text(&constant)
    }

    /// Format one signed or unsigned 128 bit constant.
    fn format_wide_constant(&mut self, opcode: Opcode) -> FormatResult<()> {
        let ty = if opcode == Opcode::CONSTANT_INT128 {
            ValueType::int128()
        } else {
            ValueType::uint128()
        };
        self.result_range(ty)?;

        // preserve the declared signedness in source text
        let bits = self.u128()?;
        let value = if opcode == Opcode::CONSTANT_INT128 {
            (bits as i128).to_string()
        } else {
            bits.to_string()
        };
        write!(self.formatter, [space(), token("="), space()])?;
        self.write_text(&value)
    }

    /// Format one nullish pointer or reference constant.
    fn format_nullish(&mut self, opcode: Opcode) -> FormatResult<()> {
        let result = self.register_id()?;
        let ty = self.value_type()?;
        self.write_result(result, ty)?;
        let literal = if opcode == Opcode::CONSTANT_NULL {
            "null"
        } else {
            "undefined"
        };

        write!(self.formatter, [space(), token("="), space()])?;
        self.write_text(literal)
    }

    /// Format one storage initialization value.
    fn format_storage_constant(&mut self, opcode: Opcode) -> FormatResult<()> {
        self.declared_result_range()?;
        let literal = if opcode == Opcode::CONSTANT_UNINIT {
            "uninit"
        } else {
            "zeroed"
        };

        write!(self.formatter, [space(), token("="), space()])?;
        self.write_text(literal)
    }

    /// Format one scalar constant.
    pub(super) fn format_constant(&mut self, scalar: Scalar) -> FormatResult<()> {
        self.result(ValueType::scalar(scalar))?;
        let bits = self.u64()?;
        let literal = scalar.literal(bits);

        // write the canonical literal
        write!(self.formatter, [space(), token("="), space()])?;
        self.write_text(&literal)?;

        Ok(())
    }

    /// Format one boolean operation.
    pub(super) fn format_boolean(&mut self, operation: BooleanOperation) -> FormatResult<()> {
        let name = format!("int.{}", operation.name());
        self.format_scalar_operation(&name, &[ValueType::scalar(Scalar::Boolean)])
    }

    /// Format one regular scalar operation.
    pub(super) fn format_scalar(&mut self, operation: &str, scalar: Scalar) -> FormatResult<()> {
        let opcode = self.instruction.opcode();
        let integer = opcode.integer_operation().map(|(operation, _)| operation);
        let float = opcode.float_operation().map(|(operation, _)| operation);
        let result = if integer.is_some_and(IntegerOperation::is_comparison)
            || float.is_some_and(FloatOperation::is_comparison)
        {
            ValueType::scalar(Scalar::Boolean)
        } else if integer.is_some_and(IntegerOperation::is_count) {
            ValueType::scalar(Scalar::Uint32)
        } else {
            ValueType::scalar(scalar)
        };
        let results = if integer.is_some_and(IntegerOperation::is_overflowing) {
            vec![result, ValueType::scalar(Scalar::Boolean)]
        } else {
            vec![result]
        };
        let prefix = if scalar.is_float() { "float" } else { "int" };

        let name = format!("{prefix}.{operation}");

        self.format_scalar_operation(&name, &results)
    }

    /// Format one 128-bit integer operation.
    pub(super) fn format_integer128(
        &mut self,
        operation: IntegerOperation,
        is_signed: bool,
    ) -> FormatResult<()> {
        let scalar = if is_signed {
            ValueType::int128()
        } else {
            ValueType::uint128()
        };
        let primary = if operation.is_comparison() {
            ValueType::scalar(Scalar::Boolean)
        } else if operation.is_count() {
            ValueType::scalar(Scalar::Uint32)
        } else {
            scalar
        };
        let results = if operation.is_overflowing() {
            vec![primary, ValueType::scalar(Scalar::Boolean)]
        } else {
            vec![primary]
        };
        let name = format!("int.{}", operation.name());

        self.format_scalar_operation(&name, &results)
    }

    /// Format one scalar cast.
    pub(super) fn format_cast(
        &mut self,
        operation: CastOperation,
        source: ValueType,
        target: ValueType,
    ) -> FormatResult<()> {
        self.result(target)?;
        let input = self.register_id()?;
        let operation = operation
            .name(source, target)
            .ok_or(FormatError::SyntaxError {
                message: "cast has no canonical name",
            })?;

        // write the typed conversion
        write!(
            self.formatter,
            [space(), token("="), space(), token("cast.")]
        )?;
        self.write_text(operation)?;
        self.write_token(" ")?;
        self.write_register(input)?;
        write!(self.formatter, [space(), token("->"), space()])?;
        write!(self.formatter, [target])
    }

    /// Format one regular scalar operation from its exact operand layout.
    fn format_scalar_operation(
        &mut self,
        name: &str,
        result_types: &[ValueType],
    ) -> FormatResult<()> {
        let layout = self
            .instruction
            .opcode()
            .layout()
            .ok_or(FormatError::SyntaxError {
                message: "scalar opcode has no operand layout",
            })?;
        let mut result_types = result_types.iter().copied();
        let mut results = Vec::new();
        let mut arguments = Vec::new();

        // decode results and arguments from the exact opcode layout
        for operand in layout.operands() {
            match operand {
                Operand::Result => {
                    let ty = result_types.next().ok_or(FormatError::SyntaxError {
                        message: "scalar result has no value type",
                    })?;
                    let register = self.register_id()?;
                    results.push((register, ty));
                }
                Operand::ResultRange => {
                    let ty = result_types.next().ok_or(FormatError::SyntaxError {
                        message: "scalar result has no value type",
                    })?;
                    let (register, word_count) = self.register_range_id()?;
                    if word_count != ty.word_count() {
                        return Err(FormatError::SyntaxError {
                            message: "scalar result width does not match its type",
                        });
                    }
                    results.push((register, ty));
                }
                Operand::Register => arguments.push(self.register_id()?),
                Operand::RegisterList => {
                    let count = self.u16()?;
                    for _ in 0..count {
                        arguments.push(self.register_id()?);
                    }
                }
                Operand::RegisterRange => {
                    let (register, word_count) = self.register_range_id()?;
                    if word_count > 0 {
                        let ty = self.formatter.context().register_type(register)?;
                        if word_count != ty.word_count() {
                            return Err(FormatError::SyntaxError {
                                message: "scalar argument width does not match its type",
                            });
                        }
                        arguments.push(register);
                    }
                }
                _ => {
                    return Err(FormatError::SyntaxError {
                        message: "scalar operation has a specialized operand",
                    });
                }
            }
        }
        if result_types.next().is_some() {
            return Err(FormatError::SyntaxError {
                message: "scalar operation has unused result types",
            });
        }

        // write every typed result before the operation
        for (index, (register, ty)) in results.iter().copied().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_result(register, ty)?;
        }
        if !results.is_empty() {
            write!(self.formatter, [space(), token("="), space()])?;
        }

        // write the operation and ordered register arguments
        self.write_text(name)?;
        for (index, argument) in arguments.into_iter().enumerate() {
            if index == 0 {
                write!(self.formatter, [space()])?;
            } else {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_register(argument)?;
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
                    unreachable!("floating-point scalars have one concrete format");
                };

                Self::float_literal(value, Word::scalar(bits, self).bits())
            }
        }
    }

    /// Format one floating-point value accepted by the assembler.
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
