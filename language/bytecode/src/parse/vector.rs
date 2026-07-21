use crate::{
    ConvertMode, FloatOperation, InstructionBuilder, IntegerOperation, Opcode, ParseError,
    ParseResult, Parser, ReduceOperation, RegisterId, RegisterRange, Scalar, Token, TokenType,
    ValueType, VectorOperation, VectorType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one fixed width vector instruction.
    pub(super) fn parse_vector_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "splat" => self.parse_vector_splat(token, results, result_types, function),
            "insert" => self.parse_vector_insert(token, results, result_types, function),
            "extract" => self.parse_vector_extract(token, results, function),
            "shuffle" => self.parse_vector_shuffle(token, results, result_types, function),
            "compare" => self.parse_vector_compare(token, results, function),
            "select" => self.parse_vector_select(token, results, result_types, function),
            "reduce" => self.parse_vector_reduce(token, results, function),
            "convert" => self.parse_vector_convert(token, results, result_types, function),
            "load" => self.parse_vector_load(token, results, result_types, function),
            _ => self.parse_vector_element(name, token, results, result_types, function),
        }
    }

    /// Parse one scalar splat across every vector lane.
    fn parse_vector_splat(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let vector = self.vector_result(result_types, token)?;
        let value = self.parse_register()?;
        if !function.has_type(value, ValueType::scalar(vector.scalar)) {
            return Err(ParseError::new(
                "vector splat input does not match its lane type",
                token.span,
            ));
        }

        // encode the scalar input and wide vector result
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Splat));
        instruction.register(value);
        instruction.vector_type(vector);

        function.emit(
            instruction,
            results,
            &[ValueType::vector(vector)],
            self.empty_span(),
        )
    }

    /// Parse one scalar lane insertion.
    fn parse_vector_insert(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let vector = self.vector_result(result_types, token)?;
        let vector_type = ValueType::vector(vector);
        let input = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;

        // match the vector, lane index, and scalar lane representations
        let inputs = [input, index, value];
        let types = [
            vector_type,
            ValueType::scalar(Scalar::Uint32),
            ValueType::scalar(vector.scalar),
        ];
        if !function.values_match(&inputs, &types) {
            return Err(ParseError::new(
                "vector insert inputs do not match its vector type",
                token.span,
            ));
        }

        // encode the three inputs and wide vector result
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Insert));
        instruction.range(RegisterRange::new(input, vector.word_count()));
        instruction.register(index);
        instruction.register(value);
        instruction.vector_type(vector);

        function.emit(instruction, results, &[vector_type], self.empty_span())
    }

    /// Parse one scalar lane extraction.
    fn parse_vector_extract(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let input = self.parse_register()?;
        let vector = self.vector_type(function, input, token)?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_register()?;
        if !function.has_type(index, ValueType::scalar(Scalar::Uint32)) {
            return Err(ParseError::new(
                "vector extract index is not uint32",
                token.span,
            ));
        }

        // encode the vector range and lane index
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Extract));
        instruction.range(RegisterRange::new(input, vector.word_count()));
        instruction.register(index);
        instruction.vector_type(vector);

        function.emit(
            instruction,
            results,
            &[ValueType::scalar(vector.scalar)],
            self.empty_span(),
        )
    }

    /// Parse one two-input lane shuffle.
    fn parse_vector_shuffle(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let vector = self.vector_result(result_types, token)?;
        let vector_type = ValueType::vector(vector);
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        self.eat_token(TokenType::OpenBracket)?;
        let lanes = self.parse_u16_list(TokenType::CloseBracket)?;

        // match both vectors and one output lane per result lane
        if !function.values_match(&[left, right], &[vector_type, vector_type]) {
            return Err(ParseError::new(
                "vector shuffle inputs do not match its result type",
                token.span,
            ));
        }
        let lane_limit = u32::from(vector.lane_count) * 2;
        let are_lanes_valid = lanes.len() == vector.lane_count as usize
            && lanes.iter().all(|lane| u32::from(*lane) < lane_limit);
        if !are_lanes_valid {
            return Err(ParseError::new("invalid vector shuffle lanes", token.span));
        }

        // encode the inputs, lane map, and vector representation
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Shuffle));
        instruction
            .registers(&[left, right])
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.u16(vector.lane_count);
        for lane in lanes {
            instruction.u16(lane);
        }
        instruction.vector_type(vector);

        function.emit(instruction, results, &[vector_type], self.empty_span())
    }

    /// Parse one lane comparison.
    fn parse_vector_compare(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operator = self.eat_token(TokenType::Identifier)?;
        let operator_name = self.text(operator).to_string();
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_register()?;
        let vector = self.vector_type(function, left, token)?;
        let operator = self.vector_operator(&operator_name, vector.scalar, operator)?;
        let vector_type = ValueType::vector(vector);
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        if !function.has_type(right, vector_type) {
            return Err(ParseError::new(
                "vector comparison inputs do not match",
                token.span,
            ));
        }

        // encode the matching vector inputs and boolean mask result
        let mask = vector.mask();
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Compare));
        instruction
            .registers(&[left, right])
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.vector_type(vector);
        instruction.u16(operator);

        function.emit(
            instruction,
            results,
            &[ValueType::vector(mask)],
            self.empty_span(),
        )
    }

    /// Parse one lane selection.
    fn parse_vector_select(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let vector = self.vector_result(result_types, token)?;
        let vector_type = ValueType::vector(vector);
        let condition = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;

        // match one boolean mask and two vector inputs
        let inputs = [condition, left, right];
        let types = [ValueType::vector(vector.mask()), vector_type, vector_type];
        if !function.values_match(&inputs, &types) {
            return Err(ParseError::new(
                "vector selection inputs do not match its result type",
                token.span,
            ));
        }

        // encode all input values and the wide result
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Select));
        instruction
            .registers(&[condition, left, right])
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.vector_type(vector);

        function.emit(instruction, results, &[vector_type], self.empty_span())
    }

    /// Parse one vector reduction to a scalar.
    fn parse_vector_reduce(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operator = self.eat_token(TokenType::Identifier)?;
        let (prefix, operation) = self
            .text(operator)
            .split_once('.')
            .ok_or_else(|| ParseError::new("expected vector reduction", operator.span))?;
        self.eat_token(TokenType::Comma)?;
        let input = self.parse_register()?;
        let vector = self.vector_type(function, input, token)?;
        let expected_prefix = if vector.scalar.is_float() {
            "float"
        } else {
            "int"
        };
        let operation = ReduceOperation::from_name(operation)
            .filter(|_| prefix == expected_prefix)
            .ok_or_else(|| ParseError::new("expected vector reduction", operator.span))?;

        // encode the wide input and scalar result
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Reduce));
        instruction.range(RegisterRange::new(input, vector.word_count()));
        instruction.vector_type(vector);
        instruction.u16(operation as u16);

        function.emit(
            instruction,
            results,
            &[ValueType::scalar(vector.scalar)],
            self.empty_span(),
        )
    }

    /// Parse one lane representation conversion.
    fn parse_vector_convert(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let target = self.vector_result(result_types, token)?;
        let mode = self.eat_token(TokenType::Identifier)?;
        let mode = ConvertMode::from_name(self.text(mode))
            .ok_or_else(|| ParseError::new("expected vector conversion mode", mode.span))?;
        self.eat_token(TokenType::Comma)?;
        let input = self.parse_register()?;
        let source = self.vector_type(function, input, token)?;
        if source.lane_count != target.lane_count {
            return Err(ParseError::new(
                "vector conversion must preserve its lane count",
                token.span,
            ));
        }

        // encode source and target representations around the input range
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Convert));
        instruction.range(RegisterRange::new(input, source.word_count()));
        instruction.vector_type(source);
        instruction.vector_type(target);
        instruction.u16(mode as u16);

        function.emit(
            instruction,
            results,
            &[ValueType::vector(target)],
            self.empty_span(),
        )
    }

    /// Parse one vector load.
    fn parse_vector_load(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let vector = self.vector_result(result_types, token)?;
        let pointer = self.parse_register()?;
        if !function.has_type(pointer, ValueType::pointer()) {
            return Err(ParseError::new(
                "vector load target is not a pointer",
                token.span,
            ));
        }

        // encode the pointer and wide vector result
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Load));
        instruction.register(pointer);
        instruction.vector_type(vector);

        function.emit(
            instruction,
            results,
            &[ValueType::vector(vector)],
            self.empty_span(),
        )
    }

    /// Parse one elementwise vector operation.
    fn parse_vector_element(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let vector = self.vector_result(result_types, token)?;
        let vector_type = ValueType::vector(vector);

        // resolve the scalar operation and exact input count
        let (prefix, operation_name) = name
            .split_once('.')
            .ok_or_else(|| ParseError::new("expected vector operation", token.span))?;
        let expected_prefix = if vector.scalar.is_float() {
            "float"
        } else {
            "int"
        };
        if prefix != expected_prefix {
            return Err(ParseError::new(
                "vector operation does not match its scalar representation",
                token.span,
            ));
        }
        let operator = if vector.scalar.is_float() {
            FloatOperation::from_name(operation_name)
                .map(|operation| (operation as u16, operation.input_count()))
        } else {
            IntegerOperation::from_name(operation_name)
                .map(|operation| (operation as u16, operation.input_count()))
        }
        .ok_or_else(|| ParseError::new("expected vector operation", token.span))?;
        let inputs = self.parse_exact_registers(operator.1)?;
        let are_inputs_matching = inputs
            .iter()
            .all(|input| function.has_type(*input, vector_type));
        if !are_inputs_matching {
            return Err(ParseError::new(
                "vector operands do not match its result type",
                token.span,
            ));
        }

        // encode the input vectors and scalar operation code
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Element));
        instruction
            .registers(&inputs)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.vector_type(vector);
        instruction.u16(operator.0);

        function.emit(instruction, results, &[vector_type], self.empty_span())
    }

    /// Return the declared vector result representation.
    fn vector_result(&self, result_types: &[ValueType], token: Token) -> ParseResult<VectorType> {
        result_types
            .first()
            .and_then(|ty| ty.vector_type())
            .ok_or_else(|| ParseError::new("expected vector result", token.span))
    }

    /// Return one initialized vector register's representation.
    fn vector_type(
        &self,
        function: &FunctionParser,
        register: RegisterId,
        token: Token,
    ) -> ParseResult<VectorType> {
        function
            .value_type(register)
            .and_then(ValueType::vector_type)
            .ok_or_else(|| ParseError::new("expected vector input", token.span))
    }

    /// Resolve one vector operator for its exact scalar representation.
    fn vector_operator(&self, text: &str, scalar: Scalar, token: Token) -> ParseResult<u16> {
        let (prefix, name) = text
            .split_once('.')
            .ok_or_else(|| ParseError::new("expected vector comparison", token.span))?;
        let expected_prefix = if scalar.is_float() { "float" } else { "int" };
        if prefix != expected_prefix {
            return Err(ParseError::new("expected vector comparison", token.span));
        }

        // accept comparisons from the exact scalar representation
        let operation = match expected_prefix {
            "int" => IntegerOperation::from_name(name)
                .filter(|operation| operation.is_comparison())
                .map(|operation| operation as u16),
            "float" => FloatOperation::from_name(name)
                .filter(|operation| operation.is_comparison())
                .map(|operation| operation as u16),
            _ => None,
        };

        operation.ok_or_else(|| ParseError::new("expected vector comparison", token.span))
    }
}
