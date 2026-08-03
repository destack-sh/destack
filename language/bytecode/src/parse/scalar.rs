use crate::{
    BooleanOperation, CastOperation, FloatOperation, InstructionBuilder, IntegerOperation, Opcode,
    ParseError, ParseResult, Parser, RegisterSpan, RelocationTag, Scalar, Token, TokenType, TypeId,
    ValueTag, ValueType,
};

use super::function::FunctionParser;

/// One scalar literal consumed before its trailing representation.
#[derive(Clone, Copy, Debug)]
struct ScalarLiteral {
    /// The literal payload token.
    token: Token,
    /// Whether the payload is an exact floating point bit pattern.
    is_bits: bool,
}

impl Parser<'_> {
    /// Parse one constant instruction.
    pub(super) fn parse_constant_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        if name != "constant" {
            return Err(ParseError::new("unknown constant operation", token.span));
        }

        let results = self.parse_results(1, true)?;

        // parse constants whose representation is intrinsic
        let opcode = if self.eat_name_if("null") {
            Some(Opcode::CONSTANT_NULL)
        } else if self.eat_name_if("undefined") {
            Some(Opcode::CONSTANT_UNDEFINED)
        } else if self.eat_name_if("zeroed") {
            Some(Opcode::CONSTANT_ZEROED)
        } else {
            None
        };
        if let Some(opcode) = opcode {
            let instruction = InstructionBuilder::new(opcode);

            return function.emit(instruction, &results, token.span);
        }

        // parse the literal before selecting its concrete opcode
        let literal = self.parse_scalar_literal()?;
        let ty = self.parse_representation()?;
        let instruction = match ty.tag() {
            ValueTag::SCALAR => {
                let scalar = ty.scalar_type().ok_or_else(|| {
                    ParseError::new("invalid scalar representation", literal.token.span)
                })?;
                let bits = self.scalar_bits(literal, scalar)?;
                let mut instruction = InstructionBuilder::new(Opcode::constant(scalar));
                instruction.u64(bits);

                instruction
            }
            ValueTag::INT128 | ValueTag::UINT128 => {
                let bits = self.wide_bits(literal, ty)?;
                let opcode = if ty.tag() == ValueTag::INT128 {
                    Opcode::CONSTANT_INT128
                } else {
                    Opcode::CONSTANT_UINT128
                };
                let mut instruction = InstructionBuilder::new(opcode);
                instruction.u128(bits);

                instruction
            }
            ValueTag::TYPE_ID => {
                let ty = TypeId::from_name(self.text(literal.token))
                    .ok_or_else(|| ParseError::new("expected type id", literal.token.span))?;
                let mut instruction = InstructionBuilder::new(Opcode::CONSTANT_TYPE);
                instruction.relocation(RelocationTag::TYPE, ty.0);

                instruction
            }
            _ => {
                return Err(ParseError::new(
                    "constant requires a scalar or type id representation",
                    token.span,
                ));
            }
        };

        function.emit(instruction, &results, token.span)
    }

    /// Parse one literal token or exact floating point bit pattern.
    fn parse_scalar_literal(&mut self) -> ParseResult<ScalarLiteral> {
        if self.eat_name_if("bits") {
            self.eat_token(TokenType::OpenParenthesis)?;
            let token = self.eat_token(TokenType::Integer)?;
            self.eat_token(TokenType::CloseParenthesis)?;

            Ok(ScalarLiteral {
                token,
                is_bits: true,
            })
        } else {
            Ok(ScalarLiteral {
                token: self.bump(),
                is_bits: false,
            })
        }
    }

    /// Decode one scalar literal into canonical register bits.
    fn scalar_bits(&self, literal: ScalarLiteral, scalar: Scalar) -> ParseResult<u64> {
        let text = self.text(literal.token).replace('_', "");
        if literal.is_bits {
            if !scalar.is_float() {
                return Err(ParseError::new(
                    "exact bits require a floating point representation",
                    literal.token.span,
                ));
            }
            let text = text.strip_prefix("0x").ok_or_else(|| {
                ParseError::new(
                    "expected hexadecimal floating point bits",
                    literal.token.span,
                )
            })?;

            return u64::from_str_radix(text, 16)
                .map_err(|_| ParseError::new("invalid floating point bits", literal.token.span));
        }
        if scalar == Scalar::Boolean {
            return match text.as_str() {
                "true" => Ok(1),
                "false" => Ok(0),
                _ => Err(ParseError::new(
                    "expected boolean literal",
                    literal.token.span,
                )),
            };
        }
        if scalar.is_integer() {
            return if text.starts_with('-') {
                text.parse::<i64>()
                    .map(|value| value as u64)
                    .map_err(|_| ParseError::new("expected integer literal", literal.token.span))
            } else {
                text.parse::<u64>()
                    .map_err(|_| ParseError::new("expected integer literal", literal.token.span))
            };
        }

        // preserve canonical names for non-finite values
        let value = match text.as_str() {
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            "NaN" => f64::NAN,
            _ => text.parse::<f64>().map_err(|_| {
                ParseError::new("expected floating point literal", literal.token.span)
            })?,
        };

        scalar.float_bits(value).ok_or_else(|| {
            ParseError::new("expected floating point representation", literal.token.span)
        })
    }

    /// Decode one signed or unsigned 128 bit literal.
    fn wide_bits(&self, literal: ScalarLiteral, ty: ValueType) -> ParseResult<u128> {
        if literal.is_bits {
            return Err(ParseError::new(
                "128 bit integers do not accept floating point bits",
                literal.token.span,
            ));
        }

        let text = self.text(literal.token).replace('_', "");
        if ty.tag() == ValueTag::INT128 {
            text.parse::<i128>()
                .map(|value| value as u128)
                .map_err(|_| ParseError::new("expected 128 bit integer", literal.token.span))
        } else {
            text.parse::<u128>()
                .map_err(|_| ParseError::new("expected 128 bit integer", literal.token.span))
        }
    }

    /// Parse one scalar cast instruction.
    pub(super) fn parse_cast_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operation = name
            .strip_prefix("cast.")
            .and_then(CastOperation::from_name)
            .ok_or_else(|| ParseError::new("unknown cast operation", token.span))?;
        let results = self.parse_results(1, true)?;
        let input = self.parse_register_span()?;
        let source = self.parse_representation()?;
        self.eat_token(TokenType::Arrow)?;
        let target = self.parse_value_type()?;
        let opcode = Opcode::cast(operation, source, target)
            .ok_or_else(|| ParseError::new("invalid cast", token.span))?;
        if input.word_count != 1 {
            return Err(ParseError::new(
                "scalar cast requires one input register",
                token.span,
            ));
        }

        // encode the selected scalar conversion
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(input.start);

        function.emit(instruction, &results, token.span)
    }

    /// Parse one boolean, integer, or floating point operation.
    pub(super) fn parse_numeric_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (family, name) = name
            .split_once('.')
            .ok_or_else(|| ParseError::new("expected scalar operation", token.span))?;

        // boolean operations have one fixed representation
        if family == "boolean" {
            let operation = BooleanOperation::from_name(name)
                .ok_or_else(|| ParseError::new("unknown boolean operation", token.span))?;
            let results = self.parse_results(1, true)?;
            let inputs = self.parse_input_spans(operation.input_count())?;
            let mut instruction = InstructionBuilder::new(Opcode::boolean(operation));
            for input in inputs {
                if input.word_count != 1 {
                    return Err(ParseError::new(
                        "boolean operation requires single registers",
                        token.span,
                    ));
                }
                instruction.register(input.start);
            }

            return function.emit(instruction, &results, token.span);
        }

        // parse destinations and inputs before the trailing representation
        let (integer, float, input_count, result_count) = match family {
            "int" => {
                let operation = IntegerOperation::from_name(name)
                    .ok_or_else(|| ParseError::new("unknown integer operation", token.span))?;
                let result_count = if operation.is_overflowing() { 2 } else { 1 };

                (Some(operation), None, operation.input_count(), result_count)
            }
            "float" => {
                let operation = FloatOperation::from_name(name).ok_or_else(|| {
                    ParseError::new("unknown floating point operation", token.span)
                })?;

                (None, Some(operation), operation.input_count(), 1)
            }
            _ => return Err(ParseError::new("unknown scalar family", token.span)),
        };
        let results = self.parse_results(result_count, true)?;
        let inputs = self.parse_input_spans(input_count)?;
        let ty = self.parse_representation()?;

        // select and encode the concrete machine representation
        let opcode = self.resolve_numeric_opcode(integer, float, ty, token)?;
        let mut instruction = InstructionBuilder::new(opcode);
        let is_wide = matches!(ty.tag(), ValueTag::INT128 | ValueTag::UINT128);
        for (index, input) in inputs.into_iter().enumerate() {
            if is_wide && !integer.is_some_and(|operation| operation.uses_count() && index == 1) {
                instruction.span(input);
            } else {
                if input.word_count != 1 {
                    return Err(ParseError::new(
                        "scalar operation requires single registers",
                        token.span,
                    ));
                }
                instruction.register(input.start);
            }
        }

        function.emit(instruction, &results, token.span)
    }

    /// Parse one exact number of comma-separated input spans.
    fn parse_input_spans(&mut self, count: usize) -> ParseResult<Vec<RegisterSpan>> {
        let mut inputs = Vec::with_capacity(count);
        for index in 0..count {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            inputs.push(self.parse_register_span()?);
        }

        Ok(inputs)
    }

    /// Resolve one concrete numeric opcode from its trailing representation.
    fn resolve_numeric_opcode(
        &self,
        integer: Option<IntegerOperation>,
        float: Option<FloatOperation>,
        ty: ValueType,
        token: Token,
    ) -> ParseResult<Opcode> {
        if matches!(ty.tag(), ValueTag::INT128 | ValueTag::UINT128) {
            let operation = integer.ok_or_else(|| {
                ParseError::new("wide values require an integer operation", token.span)
            })?;

            return Ok(Opcode::integer128(operation, ty.tag() == ValueTag::INT128));
        }

        let scalar = ty
            .scalar_type()
            .ok_or_else(|| ParseError::new("expected scalar representation", token.span))?;
        if let Some(operation) = integer {
            Opcode::integer(operation, scalar)
                .ok_or_else(|| ParseError::new("invalid integer representation", token.span))
        } else {
            let operation = float
                .ok_or_else(|| ParseError::new("expected floating point operation", token.span))?;
            Opcode::float(operation, scalar)
                .ok_or_else(|| ParseError::new("invalid floating point representation", token.span))
        }
    }
}
