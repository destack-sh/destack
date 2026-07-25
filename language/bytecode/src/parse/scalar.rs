use crate::{
    BooleanOperation, CastOperation, FloatOperation, InstructionBuilder, IntegerOperation, Opcode,
    ParseError, ParseResult, Parser, RegisterSpan, RelocationTag, Scalar, Token, TokenType,
    ValueTag, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one constant instruction.
    pub(super) fn parse_constant_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        if let Some(scalar) = name.strip_prefix("constant.").and_then(Scalar::from_name) {
            let opcode = Opcode::constant(scalar);
            let results = self.parse_definitions(opcode)?;
            let bits = self.parse_scalar_bits(scalar)?;
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.u64(bits);

            return function.emit(instruction, &results, self.empty_span());
        }
        if name == "constant.int128" || name == "constant.uint128" {
            let opcode = if name == "constant.int128" {
                Opcode::CONSTANT_INT128
            } else {
                Opcode::CONSTANT_UINT128
            };
            let results = self.parse_definitions(opcode)?;
            let literal = self.eat_token(TokenType::Integer)?;
            let text = self.text(literal).replace('_', "");
            let bits = if name == "constant.int128" {
                text.parse::<i128>().map(|value| value as u128)
            } else {
                text.parse::<u128>()
            }
            .map_err(|_| ParseError::new("expected 128 bit integer", literal.span))?;
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.u128(bits);

            return function.emit(instruction, &results, self.empty_span());
        }
        if name == "constant.type" {
            let results = self.parse_definitions(Opcode::CONSTANT_TYPE)?;
            let ty = self.parse_type_id()?;
            let mut instruction = InstructionBuilder::new(Opcode::CONSTANT_TYPE);
            instruction.relocation(RelocationTag::TYPE, ty.0);

            return function.emit(instruction, &results, self.empty_span());
        }
        let opcode = match name {
            "constant.null" => Some(Opcode::CONSTANT_NULL),
            "constant.undefined" => Some(Opcode::CONSTANT_UNDEFINED),
            "constant.zeroed" => Some(Opcode::CONSTANT_ZEROED),
            _ => None,
        };
        if let Some(opcode) = opcode {
            let results = self.parse_definitions(opcode)?;
            let instruction = InstructionBuilder::new(opcode);

            return function.emit(instruction, &results, self.empty_span());
        }
        Err(ParseError::new("unknown constant operation", token.span))
    }

    /// Parse one scalar constant into its canonical register bits.
    fn parse_scalar_bits(&mut self, scalar: Scalar) -> ParseResult<u64> {
        if scalar == Scalar::Boolean {
            let token = self.eat_token(TokenType::Identifier)?;

            return match self.text(token) {
                "true" => Ok(1),
                "false" => Ok(0),
                _ => Err(ParseError::new("expected boolean literal", token.span)),
            };
        }
        if scalar.is_integer() {
            let token = self.eat_token(TokenType::Integer)?;
            let text = self.text(token).replace('_', "");
            let bits = if text.starts_with('-') {
                text.parse::<i64>()
                    .map(|value| value as u64)
                    .map_err(|_| ParseError::new("expected integer literal", token.span))?
            } else {
                text.parse::<u64>()
                    .map_err(|_| ParseError::new("expected integer literal", token.span))?
            };

            return Ok(bits);
        }

        // preserve an explicit floating point bit pattern
        if self.eat_name_if("bits") {
            self.eat_token(TokenType::OpenParenthesis)?;
            let token = self.eat_token(TokenType::Integer)?;
            let text = self.text(token).replace('_', "");
            let text = text.strip_prefix("0x").ok_or_else(|| {
                ParseError::new("expected hexadecimal floating point bits", token.span)
            })?;
            let bits = u64::from_str_radix(text, 16)
                .map_err(|_| ParseError::new("invalid floating point bits", token.span))?;
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(bits);
        }

        let token = self.bump();
        let text = self.text(token).replace('_', "");
        let value = match text.as_str() {
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            "NaN" => f64::NAN,
            _ => text
                .parse::<f64>()
                .map_err(|_| ParseError::new("expected floating-point literal", token.span))?,
        };
        let bits = scalar
            .float_bits(value)
            .ok_or_else(|| ParseError::new("expected floating-point type", token.span))?;

        Ok(bits)
    }

    /// Parse one scalar cast instruction.
    pub(super) fn parse_cast_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (name, result_type) = name
            .rsplit_once('.')
            .ok_or_else(|| ParseError::new("cast operation has no result type", token.span))?;
        let (name, source_type) = name
            .rsplit_once('.')
            .ok_or_else(|| ParseError::new("cast operation has no source type", token.span))?;
        let source_type = self.parse_cast_type(source_type, token)?;
        let result_type = self.parse_cast_type(result_type, token)?;
        let result = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let input = self.parse_register()?;

        // resolve the exact conversion from its operation and value types
        let operation_name = name
            .strip_prefix("cast.")
            .ok_or_else(|| ParseError::new("expected cast operation", token.span))?;
        let operation = CastOperation::from_name(operation_name)
            .ok_or_else(|| ParseError::new("expected cast operation", token.span))?;
        let opcode = Opcode::cast(operation, source_type, result_type)
            .ok_or_else(|| ParseError::new("invalid cast", token.span))?;
        let results = [RegisterSpan::new(result, 1)];

        // encode the conversion over one register word
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(input);

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one scalar or pointer type selected by a cast opcode.
    fn parse_cast_type(&self, name: &str, token: Token) -> ParseResult<ValueType> {
        if let Some(scalar) = Scalar::from_name(name) {
            Ok(ValueType::scalar(scalar))
        } else if name == "int128" {
            Ok(ValueType::int128())
        } else if name == "uint128" {
            Ok(ValueType::uint128())
        } else if name == "pointer" {
            Ok(ValueType::pointer())
        } else {
            Err(ParseError::new("invalid cast type", token.span))
        }
    }

    /// Parse one integer or floating point operation selected by its values.
    pub(super) fn parse_numeric_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (operation_name, ty) = name
            .rsplit_once('.')
            .ok_or_else(|| ParseError::new("expected numeric representation", token.span))?;
        let ty = match ty {
            "int128" => ValueType::int128(),
            "uint128" => ValueType::uint128(),
            name => Scalar::from_name(name)
                .map(ValueType::scalar)
                .ok_or_else(|| ParseError::new("expected numeric representation", token.span))?,
        };
        let (prefix, operation_name) = operation_name
            .split_once('.')
            .ok_or_else(|| ParseError::new("expected numeric operation", token.span))?;

        // resolve the operation before consuming its logical inputs
        let (integer, float, input_count) =
            self.resolve_numeric_operation(prefix, operation_name, token)?;

        // encode 128 bit integer operations as contiguous register ranges
        if matches!(ty.tag(), ValueTag::INT128 | ValueTag::UINT128) {
            return self.parse_wide_integer(
                integer.ok_or_else(|| {
                    ParseError::new("128 bit values require an integer operation", token.span)
                })?,
                ty,
                input_count,
                function,
            );
        }

        // select the exact scalar opcode from its canonical suffix
        let scalar = ty
            .scalar_type()
            .ok_or_else(|| ParseError::new("expected scalar representation", token.span))?;
        let (opcode, _) =
            self.resolve_scalar_operation(operation_name, integer, float, scalar, token)?;
        let results = self.parse_definitions(opcode)?;
        let inputs = self.parse_exact_registers(input_count)?;

        // encode scalar result and input registers
        let mut instruction = InstructionBuilder::new(opcode);
        for input in inputs {
            instruction.register(input);
        }

        function.emit(instruction, &results, self.empty_span())
    }

    /// Resolve one integer or floating point operation and its input count.
    fn resolve_numeric_operation(
        &self,
        prefix: &str,
        name: &str,
        token: Token,
    ) -> ParseResult<(Option<IntegerOperation>, Option<FloatOperation>, usize)> {
        let integer = if prefix == "int" {
            IntegerOperation::from_name(name)
        } else {
            None
        };
        let float = if prefix == "float" {
            FloatOperation::from_name(name)
        } else {
            None
        };
        let input_count = integer
            .map(IntegerOperation::input_count)
            .or_else(|| float.map(FloatOperation::input_count))
            .ok_or_else(|| ParseError::new("expected numeric operation", token.span))?;

        Ok((integer, float, input_count))
    }

    /// Resolve one scalar operation and its logical result type.
    fn resolve_scalar_operation(
        &self,
        name: &str,
        integer: Option<IntegerOperation>,
        float: Option<FloatOperation>,
        scalar: Scalar,
        token: Token,
    ) -> ParseResult<(Opcode, ValueType)> {
        // boolean operation
        if scalar == Scalar::Boolean {
            let operation = BooleanOperation::from_name(name)
                .ok_or_else(|| ParseError::new("expected boolean operation", token.span))?;

            Ok((
                Opcode::boolean(operation),
                ValueType::scalar(Scalar::Boolean),
            ))
        }
        // integer operation
        else if let Some(operation) = integer {
            let result = if operation.is_comparison() {
                ValueType::scalar(Scalar::Boolean)
            } else if operation.is_count() {
                ValueType::scalar(Scalar::Uint32)
            } else {
                ValueType::scalar(scalar)
            };
            let opcode = Opcode::integer(operation, scalar)
                .ok_or_else(|| ParseError::new("invalid integer operation", token.span))?;

            Ok((opcode, result))
        }
        // floating point operation
        else {
            let operation = float
                .ok_or_else(|| ParseError::new("expected floating point operation", token.span))?;
            let result = if operation.is_comparison() {
                ValueType::scalar(Scalar::Boolean)
            } else {
                ValueType::scalar(scalar)
            };
            let opcode = Opcode::float(operation, scalar)
                .ok_or_else(|| ParseError::new("invalid floating point operation", token.span))?;

            Ok((opcode, result))
        }
    }

    /// Parse one 128 bit integer operation over contiguous register ranges.
    fn parse_wide_integer(
        &mut self,
        operation: IntegerOperation,
        ty: ValueType,
        input_count: usize,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let is_signed = ty.tag() == ValueTag::INT128;
        let opcode = Opcode::integer128(operation, is_signed);
        let results = self.parse_definitions(opcode)?;
        let mut inputs = Vec::with_capacity(input_count);
        for index in 0..input_count {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            inputs.push(self.parse_register_span()?);
        }

        // encode wide inputs as ranges and scalar counts as registers
        let mut instruction = InstructionBuilder::new(opcode);
        for (index, input) in inputs.into_iter().enumerate() {
            if operation.uses_count() && index == 1 {
                instruction.register(input.start);
            } else {
                instruction.span(input);
            }
        }
        function.emit(instruction, &results, self.empty_span())
    }
}
