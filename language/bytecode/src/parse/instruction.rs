use crate::{
    Label, ParseError, ParseResult, Parser, RegisterId, Scalar, Token, TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse and append one complete bytecode instruction.
    pub(super) fn parse_instruction(&mut self, function: &mut FunctionParser) -> ParseResult<()> {
        // parse the optional result declaration
        let (results, result_types) = self.parse_instruction_results()?;
        if self.is_literal(&result_types) {
            return self.parse_literal(&results, &result_types, function);
        }

        // parse and dispatch the operation name
        let operation = self.eat_token(TokenType::Identifier)?;
        let name = self.text(operation).to_string();
        self.parse_operation(&name, operation, &results, &result_types, function)?;

        // match every result against its declaration
        if !function.values_match(&results, &result_types) {
            return Err(ParseError::new(
                "instruction result type does not match its declaration",
                operation.span,
            ));
        }

        Ok(())
    }

    /// Parse one operation after its result declaration.
    fn parse_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // dispatch operations by their first name component
        let prefix = name.split_once('.').map_or(name, |(prefix, _)| prefix);
        match prefix {
            // constants
            "constant" => self.parse_constant_operation(name, token, results, function),

            // scalar, vector, tensor, and register values
            "int" | "float" if result_types.first().is_some_and(|ty| ty.is_tensor()) => {
                self.parse_tensor_operation(name, token, results, result_types, function)
            }
            "int" | "float"
                if result_types
                    .first()
                    .and_then(|ty| ty.vector_type())
                    .is_some() =>
            {
                self.parse_vector_operation(name, token, results, result_types, function)
            }
            "int" | "float" => {
                self.parse_numeric_operation(name, token, results, result_types, function)
            }
            "vector" => {
                let name = name
                    .strip_prefix("vector.")
                    .ok_or_else(|| ParseError::new("expected vector operation", token.span))?;
                self.parse_vector_operation(name, token, results, result_types, function)
            }
            "tensor" => self.parse_tensor_operation(name, token, results, result_types, function),
            "select"
                if result_types
                    .first()
                    .and_then(|ty| ty.vector_type())
                    .is_some() =>
            {
                self.parse_vector_operation(name, token, results, result_types, function)
            }
            "select" | "equal" => self.parse_value_operation(name, token, results, function),
            "move" if name == "move" => self.parse_value_operation(name, token, results, function),

            // addresses
            "global" | "frame" | "reference" | "pointer" => {
                self.parse_pointer_operation(name, token, results, function)
            }

            // byte ranges, prefetch, and memory
            "copy" | "move" | "fill" | "compare" | "prefetch" | "load" | "store" => {
                self.parse_memory_operation(name, token, results, result_types, function)
            }
            "atomic" => self.parse_atomic_operation(name, token, results, result_types, function),

            // function values, slices, and dynamic values
            "function" => {
                self.parse_function_operation(name, token, results, result_types, function)
            }
            "slice" => self.parse_slice_operation(name, token, results, result_types, function),
            "dynamic" => self.parse_dynamic_operation(name, token, results, result_types, function),

            // new and destruction
            "new" => self.parse_new(name, token, results, result_types, function),
            "free" | "drop" => self.parse_reference_operation(name, token, results, function),

            // address stability
            "pin" | "unpin" => self.parse_reference_operation(name, token, results, function),

            // collector protocol
            "barrier" => self.parse_reference_operation(name, token, results, function),
            "safepoint" => self.parse_control_operation(name, token, results, function),

            // calls
            "call" | "invoke" | "tail" => self.parse_call_operation(name, token, results, function),

            // control flow
            "branch" if name != "branch" => self.parse_branch(name, token, results, function),
            "jump" | "branch" | "switch" | "yield" | "return" | "trap" | "unreachable"
            | "breakpoint" => self.parse_control_operation(name, token, results, function),

            // panic and unwind
            "panic" | "unwind" | "catch" => {
                self.parse_control_operation(name, token, results, function)
            }

            // runtime checks, casts, and profile instrumentation
            "check" => self.parse_check_operation(name, token, results, function),
            "cast" => self.parse_cast_operation(name, token, results, result_types, function),
            "profile" => self.parse_profile_operation(name, token, results, function),

            _ => Err(ParseError::new("unknown bytecode operation", token.span)),
        }
    }

    /// Return whether the next token begins a typed literal assignment.
    fn is_literal(&mut self, result_types: &[ValueType]) -> bool {
        if result_types.len() != 1 {
            return false;
        }
        let token = self.peek();
        let is_number = matches!(token.ty, TokenType::Integer | TokenType::Float);
        let is_named = token.ty == TokenType::Identifier
            && matches!(
                self.text(token),
                "true" | "false" | "Infinity" | "-Infinity" | "NaN" | "bits" | "null" | "undefined"
            );

        is_number || is_named
    }

    /// Parse the typed result declarations before one instruction.
    fn parse_instruction_results(&mut self) -> ParseResult<(Vec<RegisterId>, Vec<ValueType>)> {
        if !self.is_register() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut registers = Vec::new();
        let mut types = Vec::new();
        loop {
            registers.push(self.parse_register()?);
            self.eat_token(TokenType::Colon)?;
            types.push(self.parse_value_type()?);

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::Equal)?;

        Ok((registers, types))
    }

    /// Return whether the next token starts a register result list.
    pub(super) fn is_register(&mut self) -> bool {
        let token = self.peek();

        token.ty == TokenType::Identifier
            && self.text(token).strip_prefix('r').is_some_and(|index| {
                !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
            })
    }

    /// Parse one branch label.
    pub(super) fn parse_label(&mut self) -> ParseResult<Label> {
        let token = self.eat_token(TokenType::Identifier)?;

        self.parse_label_token(token)
    }

    /// Parse one already consumed branch label token.
    pub(super) fn parse_label_token(&self, token: Token) -> ParseResult<Label> {
        let index = self
            .text(token)
            .strip_prefix('l')
            .ok_or_else(|| ParseError::new("expected branch label", token.span))?;
        let index = index
            .parse::<u32>()
            .map_err(|_| ParseError::new("expected branch label", token.span))?;

        Ok(Label(index))
    }

    /// Parse one canonical scalar type name.
    pub(super) fn parse_scalar_name(&mut self) -> ParseResult<Scalar> {
        let token = self.eat_token(TokenType::Identifier)?;

        Scalar::from_name(self.text(token))
            .ok_or_else(|| ParseError::new("expected scalar type", token.span))
    }

    /// Parse one exact number of comma-separated registers.
    pub(super) fn parse_exact_registers(&mut self, count: usize) -> ParseResult<Vec<RegisterId>> {
        let mut registers = Vec::with_capacity(count);
        for index in 0..count {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            registers.push(self.parse_register()?);
        }

        Ok(registers)
    }

    /// Parse one comma-separated unsigned 16-bit list.
    pub(super) fn parse_u16_list(&mut self, close: TokenType) -> ParseResult<Vec<u16>> {
        let mut values = Vec::new();
        while !self.eat_token_if(close) {
            values.push(self.parse_u16()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(close)?;

                break;
            }
        }

        Ok(values)
    }
}
