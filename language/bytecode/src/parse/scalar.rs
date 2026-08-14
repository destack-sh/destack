use crate::{
    BooleanOperation, CastOperation, FloatOperation, InstructionBuilder, IntegerOperation, Opcode,
    ParseError, ParseResult, Parser, RegisterSpan, RelocationTag, Scalar, Token, TokenType, TypeId,
    ValueType,
};

use super::function::FunctionParser;

/// One scalar literal and its source encoding.
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
        let results = self.parse_results(1, false)?;
        let opcode = match name {
            "constant.null" => Opcode::CONSTANT_NULL,
            "constant.undefined" => Opcode::CONSTANT_UNDEFINED,
            "constant.zeroed" => Opcode::CONSTANT_ZEROED,
            _ => return Err(ParseError::new("unknown constant operation", token.span)),
        };
        let instruction = InstructionBuilder::new(opcode);

        function.emit(instruction, &results, token.span)
    }

    /// Parse one operation selected by a scalar representation.
    pub(super) fn parse_scalar_operation(
        &mut self,
        scalar: Scalar,
        operation: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match operation {
            "constant" => self.parse_scalar_constant(scalar, token, function),
            _ if operation.starts_with("load") || operation.starts_with("store") => {
                self.parse_scalar_memory(scalar, operation, token, function)
            }
            _ if operation.starts_with("atomic.") => {
                self.parse_scalar_atomic(scalar, operation, token, function)
            }
            _ if operation.starts_with("check.") => {
                self.parse_scalar_check(scalar, operation, token, function)
            }
            _ if operation.starts_with("branch.") => {
                self.parse_branch(scalar, operation, token, function)
            }
            _ => self.parse_numeric_operation(scalar, operation, token, function),
        }
    }

    /// Parse one scalar constant.
    fn parse_scalar_constant(
        &mut self,
        scalar: Scalar,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let results = self.parse_results(1, true)?;
        let literal = self.parse_scalar_literal()?;
        let bits = self.scalar_bits(literal, scalar)?;
        let mut instruction = InstructionBuilder::new(Opcode::constant(scalar));
        instruction.u64(bits);

        function.emit(instruction, &results, token.span)
    }

    /// Parse one 128 bit integer operation.
    pub(super) fn parse_wide_operation(
        &mut self,
        operation: &str,
        representation: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let is_signed = representation == "int128";
        if operation == "constant" {
            let results = self.parse_results(1, true)?;
            let literal = self.parse_scalar_literal()?;
            let bits = self.wide_bits(literal, is_signed)?;
            let opcode = if is_signed {
                Opcode::CONSTANT_INT128
            } else {
                Opcode::CONSTANT_UINT128
            };
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.u128(bits);

            return function.emit(instruction, &results, token.span);
        }

        let operation = IntegerOperation::from_name(operation)
            .ok_or_else(|| ParseError::new("unknown wide integer operation", token.span))?;
        let result_count = if operation.is_overflowing() { 2 } else { 1 };
        let results = self.parse_results(result_count, true)?;
        let inputs = self.parse_input_spans(operation.input_count())?;
        let mut instruction = InstructionBuilder::new(Opcode::integer128(operation, is_signed));
        for (index, input) in inputs.into_iter().enumerate() {
            if operation.uses_count() && index == 1 {
                if input.word_count != 1 {
                    return Err(ParseError::new(
                        "shift count requires one register",
                        token.span,
                    ));
                }
                instruction.register(input.start);
            } else {
                instruction.span(input);
            }
        }

        function.emit(instruction, &results, token.span)
    }

    /// Parse one linked type identity constant.
    pub(super) fn parse_type_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        if name != "constant.typeId" {
            return Err(ParseError::new(
                "unknown type identity operation",
                token.span,
            ));
        }
        let results = self.parse_results(1, true)?;
        let type_token = self.eat_token(TokenType::Identifier)?;
        let ty = TypeId::from_name(self.text(type_token))
            .ok_or_else(|| ParseError::new("expected type id", type_token.span))?;
        let mut instruction = InstructionBuilder::new(Opcode::CONSTANT_TYPE);
        instruction.relocation(RelocationTag::TYPE, ty.0);

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
    fn wide_bits(&self, literal: ScalarLiteral, is_signed: bool) -> ParseResult<u128> {
        if literal.is_bits {
            return Err(ParseError::new(
                "128 bit integers do not accept floating point bits",
                literal.token.span,
            ));
        }

        let text = self.text(literal.token).replace('_', "");
        if is_signed {
            text.parse::<i128>()
                .map(|value| value as u128)
                .map_err(|_| ParseError::new("expected 128 bit integer", literal.token.span))
        } else {
            text.parse::<u128>()
                .map_err(|_| ParseError::new("expected 128 bit integer", literal.token.span))
        }
    }

    /// Parse one scalar cast instruction.
    pub(super) fn parse_cast(
        &mut self,
        operation: CastOperation,
        source: ValueType,
        target: ValueType,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let results = self.parse_results(1, true)?;
        let input = self.parse_register()?;
        let opcode = Opcode::cast(operation, source, target)
            .ok_or_else(|| ParseError::new("invalid cast", token.span))?;

        // encode the selected scalar conversion
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(input);

        function.emit(instruction, &results, token.span)
    }

    /// Parse one boolean, integer, or floating point operation.
    pub(super) fn parse_numeric_operation(
        &mut self,
        scalar: Scalar,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // boolean operations have one fixed representation
        if scalar == Scalar::Boolean {
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

        // select the operation from the scalar family
        let (integer, float, input_count, result_count) = if scalar.is_integer() {
            let operation = IntegerOperation::from_name(name)
                .ok_or_else(|| ParseError::new("unknown integer operation", token.span))?;
            let result_count = if operation.is_overflowing() { 2 } else { 1 };

            (Some(operation), None, operation.input_count(), result_count)
        } else if scalar.is_float() {
            let operation = FloatOperation::from_name(name)
                .ok_or_else(|| ParseError::new("unknown floating point operation", token.span))?;

            (None, Some(operation), operation.input_count(), 1)
        } else {
            return Err(ParseError::new("unknown scalar operation", token.span));
        };
        let results = self.parse_results(result_count, true)?;
        let inputs = self.parse_input_spans(input_count)?;

        // select and encode the concrete machine representation
        let opcode = if let Some(operation) = integer {
            Opcode::integer(operation, scalar)
                .ok_or_else(|| ParseError::new("invalid integer operation", token.span))?
        } else {
            let operation = float
                .ok_or_else(|| ParseError::new("expected floating point operation", token.span))?;
            Opcode::float(operation, scalar)
                .ok_or_else(|| ParseError::new("invalid floating point operation", token.span))?
        };
        let mut instruction = InstructionBuilder::new(opcode);
        for input in inputs {
            if input.word_count != 1 {
                return Err(ParseError::new(
                    "scalar operation requires single registers",
                    token.span,
                ));
            }
            instruction.register(input.start);
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
}
