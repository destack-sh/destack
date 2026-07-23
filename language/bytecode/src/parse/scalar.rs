use crate::{
    BooleanOperation, CastOperation, FloatOperation, InstructionBuilder, IntegerOperation, Opcode,
    ParseError, ParseResult, Parser, RegisterId, RegisterRange, Scalar, Symbol, Token, TokenType,
    ValueTag, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one literal selected by its declared result type.
    pub(super) fn parse_literal(
        &mut self,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let ty = *result_types
            .first()
            .ok_or_else(|| ParseError::new("expected literal result type", self.peek().span))?;

        // encode scalar literals in one word
        if let Some(scalar) = ty.scalar_type() {
            let bits = self.parse_scalar_bits(scalar)?;
            let mut instruction = InstructionBuilder::new(Opcode::constant(scalar));
            instruction.u64(bits);

            return function.emit(instruction, results, &[ty], self.empty_span());
        }

        // encode signed and unsigned 128 bit literals in two words
        if matches!(ty.tag(), ValueTag::INT128 | ValueTag::UINT128) {
            let literal = self.eat_token(TokenType::Integer)?;
            let text = self.text(literal).replace('_', "");
            let bits = if ty.tag() == ValueTag::INT128 {
                text.parse::<i128>().map(|value| value as u128)
            } else {
                text.parse::<u128>()
            }
            .map_err(|_| ParseError::new("expected 128 bit integer", literal.span))?;
            let opcode = if ty.tag() == ValueTag::INT128 {
                Opcode::CONSTANT_INT128
            } else {
                Opcode::CONSTANT_UINT128
            };
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.u128(bits);

            return function.emit(instruction, results, &[ty], self.empty_span());
        }

        // encode one null pointer or reference niche
        if (ty.is_pointer() || ty.is_initialized_reference()) && self.eat_name_if("null") {
            let mut instruction = InstructionBuilder::new(Opcode::CONSTANT_NULL);
            instruction.value_type(ty);

            return function.emit(instruction, results, &[ty], self.empty_span());
        }

        // encode one undefined reference niche
        if ty.is_initialized_reference() && self.eat_name_if("undefined") {
            let mut instruction = InstructionBuilder::new(Opcode::CONSTANT_UNDEFINED);
            instruction.value_type(ty);

            return function.emit(instruction, results, &[ty], self.empty_span());
        }

        // preserve storage initialization without imposing a scalar representation
        let opcode = if self.eat_name_if("uninit") {
            Some(Opcode::CONSTANT_UNINIT)
        } else if self.eat_name_if("zeroed") {
            Some(Opcode::CONSTANT_ZEROED)
        } else {
            None
        };
        if let Some(opcode) = opcode {
            let instruction = InstructionBuilder::new(opcode);

            return function.emit(instruction, results, &[ty], self.empty_span());
        }

        Err(ParseError::new(
            "literal does not match its declared result type",
            self.peek().span,
        ))
    }

    /// Parse one immutable byte-sequence constant instruction.
    pub(super) fn parse_constant_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        if name == "constant.type" {
            let ty = self.parse_type_name()?;
            let mut instruction = InstructionBuilder::new(Opcode::CONSTANT_TYPE);
            instruction.symbol(Symbol::ty(ty.0));

            return function.emit(
                instruction,
                results,
                &[ValueType::type_id()],
                self.empty_span(),
            );
        }
        if name != "constant.bytes" {
            return Err(ParseError::new("unknown constant operation", token.span));
        }
        let constant = self.eat_token(TokenType::Identifier)?;
        let constant = self
            .symbols
            .constants
            .get(self.text(constant))
            .copied()
            .ok_or_else(|| ParseError::new("unknown constant", constant.span))?;
        let types = [ValueType::pointer(), ValueType::scalar(Scalar::Uint64)];
        let mut instruction = InstructionBuilder::new(Opcode::CONSTANT_BYTES);
        instruction.symbol(Symbol::constant(constant.0));

        function.emit(instruction, results, &types, self.empty_span())
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
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let input = self.parse_register()?;
        let source_type = function
            .value_type(input)
            .ok_or_else(|| ParseError::new("cast reads an uninitialized value", token.span))?;
        let [result_type] = result_types else {
            return Err(ParseError::new("cast requires one result", token.span));
        };
        let result_type = *result_type;

        // require the explicit target to match the declared result
        self.eat_token(TokenType::Arrow)?;
        let explicit = self.parse_value_type()?;
        if explicit != result_type {
            return Err(ParseError::new(
                "cast target does not match its declared result",
                token.span,
            ));
        }

        // resolve the exact conversion from its operation and value types
        let operation_name = name
            .strip_prefix("cast.")
            .ok_or_else(|| ParseError::new("expected cast operation", token.span))?;
        let operation = CastOperation::from_name(operation_name)
            .ok_or_else(|| ParseError::new("expected cast operation", token.span))?;
        let opcode = Opcode::cast(operation, source_type, result_type)
            .ok_or_else(|| ParseError::new("invalid cast", token.span))?;

        // encode the conversion over one register word
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(input);

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one integer or floating point operation selected by its values.
    pub(super) fn parse_numeric_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (prefix, operation_name) = name
            .split_once('.')
            .ok_or_else(|| ParseError::new("expected numeric operation", token.span))?;
        let (operation_name, signedness) = operation_name
            .rsplit_once('.')
            .filter(|(_, suffix)| matches!(*suffix, "s" | "u"))
            .map_or((operation_name, None), |(name, suffix)| {
                (name, Some(suffix))
            });

        // resolve the operation before consuming its logical inputs
        let (integer, float, input_count) =
            self.resolve_numeric_operation(prefix, operation_name, token)?;
        let inputs = self.parse_exact_registers(input_count)?;
        let input_type = inputs
            .first()
            .and_then(|input| function.value_type(*input))
            .ok_or_else(|| ParseError::new("numeric operation reads no typed input", token.span))?;
        let signedness_matches = match signedness {
            Some("s") => input_type.is_signed_integer(),
            Some("u") => input_type.is_unsigned_integer(),
            None => true,
            _ => false,
        };
        if !signedness_matches {
            return Err(ParseError::new(
                "integer signedness does not match its input",
                token.span,
            ));
        }

        // encode 128 bit integer operations as contiguous register ranges
        if matches!(input_type.tag(), ValueTag::INT128 | ValueTag::UINT128) {
            return self.parse_wide_integer(
                integer.ok_or_else(|| {
                    ParseError::new("128 bit values require an integer operation", token.span)
                })?,
                input_type,
                token,
                results,
                result_types,
                &inputs,
                function,
            );
        }

        // select the exact scalar opcode from the first input value
        let scalar = input_type.scalar_type().ok_or_else(|| {
            ParseError::new("numeric operation requires scalar inputs", token.span)
        })?;
        let (opcode, result_type) =
            self.resolve_scalar_operation(operation_name, integer, float, scalar, token)?;

        // derive the exact logical result types
        let result_types_expected = if integer.is_some_and(IntegerOperation::is_overflowing) {
            vec![result_type, ValueType::scalar(Scalar::Boolean)]
        } else {
            vec![result_type]
        };

        // match every input and declared result type
        let uses_count = integer.is_some_and(IntegerOperation::uses_count);
        let inputs_match = inputs.iter().enumerate().all(|(index, input)| {
            let expected = if uses_count && index == 1 {
                ValueType::scalar(Scalar::Uint32)
            } else {
                input_type
            };

            function.has_type(*input, expected)
        });
        if !inputs_match || result_types != result_types_expected {
            return Err(ParseError::new(
                "numeric values do not match the operation",
                token.span,
            ));
        }

        // encode scalar result and input registers
        let mut instruction = InstructionBuilder::new(opcode);
        for input in inputs {
            instruction.register(input);
        }

        function.emit(
            instruction,
            results,
            &result_types_expected,
            self.empty_span(),
        )
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
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        inputs: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let is_signed = ty.tag() == ValueTag::INT128;
        let opcode = Opcode::integer128(operation, is_signed);
        let expected_results = if operation.is_comparison() {
            vec![ValueType::scalar(Scalar::Boolean)]
        } else if operation.is_count() {
            vec![ValueType::scalar(Scalar::Uint32)]
        } else if operation.is_overflowing() {
            vec![ty, ValueType::scalar(Scalar::Boolean)]
        } else {
            vec![ty]
        };
        if result_types != expected_results {
            return Err(ParseError::new(
                "128 bit results do not match the operation",
                token.span,
            ));
        }

        // match logical inputs without exposing their physical words in text
        let inputs_match = inputs.iter().enumerate().all(|(index, input)| {
            let expected = if operation.uses_count() && index == 1 {
                ValueType::scalar(Scalar::Uint32)
            } else {
                ty
            };

            function.has_type(*input, expected)
        });
        if !inputs_match {
            return Err(ParseError::new(
                "128 bit inputs do not match the operation",
                token.span,
            ));
        }

        // encode wide inputs as ranges and scalar counts as registers
        let mut instruction = InstructionBuilder::new(opcode);
        for (index, input) in inputs.iter().copied().enumerate() {
            if operation.uses_count() && index == 1 {
                instruction.register(input);
            } else {
                instruction.range(RegisterRange::new(input, ty.word_count()));
            }
        }
        function.emit(instruction, results, &expected_results, self.empty_span())
    }
}
