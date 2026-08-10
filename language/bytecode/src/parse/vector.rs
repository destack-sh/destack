use crate::{
    ConvertMode, FloatOperation, InstructionBuilder, IntegerOperation, Opcode, ParseError,
    ParseResult, Parser, ReduceOperation, RegisterSpan, Scalar, Token, TokenType, VectorOperation,
    VectorType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one fixed-width vector instruction.
    pub(super) fn parse_vector_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operation = name
            .strip_prefix("vector.")
            .ok_or_else(|| ParseError::new("expected vector operation", token.span))?;

        // conversions carry both source and target representations
        if let Some(mode) = operation.strip_prefix("convert.") {
            let results = self.parse_definitions(Opcode::vector(VectorOperation::Convert))?;

            return self.parse_vector_convert(mode, token, &results, function);
        }

        // select the semantic operation before its trailing representation
        let kind = match operation {
            "splat" => VectorOperation::Splat,
            "insert" => VectorOperation::Insert,
            "extract" => VectorOperation::Extract,
            "shuffle" => VectorOperation::Shuffle,
            "select" => VectorOperation::Select,
            "load" => VectorOperation::Load,
            "store" => VectorOperation::Store,
            operation if operation.starts_with("compare.") => VectorOperation::Compare,
            operation if operation.starts_with("reduce.") => VectorOperation::Reduce,
            _ => VectorOperation::Element,
        };
        let results = self.parse_definitions(Opcode::vector(kind))?;

        match operation {
            "splat" => self.parse_vector_splat(&results, function),
            "insert" => self.parse_vector_insert(&results, function),
            "extract" => self.parse_vector_extract(&results, function),
            "shuffle" => self.parse_vector_shuffle(token, &results, function),
            "select" => self.parse_vector_select(token, &results, function),
            "load" => self.parse_vector_load(&results, function),
            "store" => self.parse_vector_store(&results, function),
            operation => {
                if let Some(operator) = operation.strip_prefix("compare.") {
                    self.parse_vector_compare(operator, token, &results, function)
                } else if let Some(reduction) = operation.strip_prefix("reduce.") {
                    self.parse_vector_reduce(reduction, token, &results, function)
                } else {
                    self.parse_vector_element(operation, token, &results, function)
                }
            }
        }
    }

    /// Parse one scalar splat across every vector lane.
    fn parse_vector_splat(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register()?;
        let vector = self.parse_vector_representation()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Splat));
        instruction.register(value);
        instruction.vector_type(vector);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one scalar lane insertion.
    fn parse_vector_insert(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let input = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;
        let vector = self.parse_vector_representation()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Insert));
        instruction.span(input);
        instruction.register(index);
        instruction.register(value);
        instruction.vector_type(vector);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one scalar lane extraction.
    fn parse_vector_extract(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let input = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_register()?;
        let vector = self.parse_vector_representation()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Extract));
        instruction.span(input);
        instruction.register(index);
        instruction.vector_type(vector);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one two-input lane shuffle.
    fn parse_vector_shuffle(
        &mut self,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let inputs = self.parse_vector_spans(2)?;
        self.eat_token(TokenType::Comma)?;
        self.eat_token(TokenType::OpenBracket)?;
        let lanes = self.parse_u16_list(TokenType::CloseBracket)?;
        let vector = self.parse_vector_representation()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Shuffle));
        instruction
            .span_starts(&inputs)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction
            .u16s(&lanes)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.vector_type(vector);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one lane comparison.
    fn parse_vector_compare(
        &mut self,
        operator: &str,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let inputs = self.parse_vector_spans(2)?;
        let vector = self.parse_vector_representation()?;
        let operator = self.vector_operator(operator, vector.scalar, token)?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Compare));
        instruction
            .span_starts(&inputs)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.vector_type(vector);
        instruction.u16(operator);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one lane selection.
    fn parse_vector_select(
        &mut self,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let condition = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let values = self.parse_vector_spans(2)?;
        let vector = self.parse_vector_representation()?;
        let spans = [condition, values[0], values[1]];
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Select));
        instruction
            .span_starts(&spans)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.vector_type(vector);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one vector reduction to a scalar.
    fn parse_vector_reduce(
        &mut self,
        reduction: &str,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operation = ReduceOperation::from_name(reduction)
            .ok_or_else(|| ParseError::new("expected vector reduction", token.span))?;
        let input = self.parse_register_span()?;
        let vector = self.parse_vector_representation()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Reduce));
        instruction.span(input);
        instruction.vector_type(vector);
        instruction.u16(operation as u16);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one lane representation conversion.
    fn parse_vector_convert(
        &mut self,
        mode: &str,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let mode = ConvertMode::from_name(mode)
            .ok_or_else(|| ParseError::new("expected vector conversion mode", token.span))?;
        let input = self.parse_register_span()?;
        let source = self.parse_vector_representation()?;
        self.eat_token(TokenType::Arrow)?;
        let target = self.parse_vector_type()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Convert));
        instruction.span(input);
        instruction.vector_type(source);
        instruction.vector_type(target);
        instruction.u16(mode as u16);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one vector load.
    fn parse_vector_load(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        let vector = self.parse_vector_representation()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Load));
        instruction.register(pointer);
        instruction.vector_type(vector);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one vector store.
    fn parse_vector_store(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register_span()?;
        let vector = self.parse_vector_representation()?;
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Store));
        instruction.register(pointer);
        instruction.span(value);
        instruction.vector_type(vector);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one elementwise vector operation.
    fn parse_vector_element(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let inputs = self.parse_vector_spans_until_representation()?;
        let vector = self.parse_vector_representation()?;
        let operator = if vector.scalar.is_float() {
            FloatOperation::from_name(name)
                .filter(|operation| !operation.returns_boolean())
                .map(|operation| (operation as u16, operation.input_count()))
        } else {
            IntegerOperation::from_name(name)
                .filter(|operation| !operation.returns_boolean() && !operation.is_overflowing())
                .map(|operation| (operation as u16, operation.input_count()))
        }
        .ok_or_else(|| ParseError::new("expected vector operation", token.span))?;
        if inputs.len() != operator.1 {
            return Err(ParseError::new("invalid vector input count", token.span));
        }
        let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Element));
        instruction
            .span_starts(&inputs)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.vector_type(vector);
        instruction.u16(operator.0);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one exact number of comma-separated vector spans.
    fn parse_vector_spans(&mut self, count: usize) -> ParseResult<Vec<RegisterSpan>> {
        let mut spans = Vec::with_capacity(count);
        for index in 0..count {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            spans.push(self.parse_register_span()?);
        }

        Ok(spans)
    }

    /// Parse vector inputs up to their trailing representation.
    fn parse_vector_spans_until_representation(&mut self) -> ParseResult<Vec<RegisterSpan>> {
        let mut spans = vec![self.parse_register_span()?];
        while self.eat_token_if(TokenType::Comma) {
            spans.push(self.parse_register_span()?);
        }

        Ok(spans)
    }

    /// Parse one trailing vector representation.
    fn parse_vector_representation(&mut self) -> ParseResult<VectorType> {
        self.eat_token(TokenType::Colon)?;

        self.parse_vector_type()
    }

    /// Parse one vector value type without its separator.
    fn parse_vector_type(&mut self) -> ParseResult<VectorType> {
        self.parse_value_type()?
            .vector_type()
            .ok_or_else(|| ParseError::new("expected vector representation", self.previous().span))
    }

    /// Resolve one vector operator for its scalar representation.
    fn vector_operator(&self, text: &str, scalar: Scalar, token: Token) -> ParseResult<u16> {
        let operation = if scalar.is_float() {
            FloatOperation::from_name(text)
                .filter(|operation| operation.returns_boolean() && operation.input_count() == 2)
                .map(|operation| operation as u16)
        } else {
            IntegerOperation::from_name(text)
                .filter(|operation| operation.returns_boolean() && operation.input_count() == 2)
                .map(|operation| operation as u16)
        };

        operation.ok_or_else(|| ParseError::new("expected vector comparison", token.span))
    }
}
