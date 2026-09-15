use crate::{
    CastOperation, Label, Opcode, Operand, ParseError, ParseResult, Parser, RegisterSpan, Scalar,
    Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse and append one complete bytecode instruction.
    pub(super) fn parse_instruction(&mut self, function: &mut FunctionParser) -> ParseResult<()> {
        // parse and dispatch the operation name
        let operation = self.eat_token(TokenType::Identifier)?;
        let name = self.text(operation).to_string();
        self.parse_operation(&name, operation, function)
    }

    /// Parse one operation after its result declaration.
    fn parse_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // decode scalar conversions before regular scalar families
        let prefix = name.split_once('.').map_or(name, |(prefix, _)| prefix);
        if let Some((operation, source, target)) = CastOperation::parse(name) {
            return self.parse_cast(operation, source, target, token, function);
        }
        if name == "constant.typeId" {
            return self.parse_type_operation(name, token, function);
        }

        // decode narrowing checks with explicit source and target representations
        if let Some(representations) = name.strip_prefix("check.narrow.")
            && let Some((source, target)) = representations.split_once('.')
            && let (Some(source), Some(_)) = (Scalar::from_name(source), Scalar::from_name(target))
        {
            let operation = format!("check.narrow.{target}");

            return self.parse_scalar_operation(source, &operation, token, function);
        }

        // select concrete scalar operations by their final representation
        if let Some((operation, representation)) = name.rsplit_once('.') {
            if let Some(scalar) = Scalar::from_name(representation) {
                return self.parse_scalar_operation(scalar, operation, token, function);
            }
            if representation == "int128" || representation == "uint128" {
                return self.parse_wide_operation(operation, representation, token, function);
            }
        }

        // dispatch the remaining operations by their first name component
        match prefix {
            // constants
            "constant" => self.parse_constant_operation(name, token, function),

            // vector and register values
            "vector" => self.parse_vector_operation(name, token, function),
            "select" | "equal" => self.parse_value_operation(name, token, function),
            "move" if name == "move" => self.parse_value_operation(name, token, function),

            // aggregates
            "aggregate" | "extract" | "insert" | "variant" => {
                self.parse_aggregate_operation(name, token, function)
            }

            // addresses and pointers
            "frame" | "global" => self.parse_address(name, token, function),
            "address" => self.parse_address_arithmetic(name, token, function),

            // memory ranges, prefetch, and scalar memory
            "memory" | "prefetch" => self.parse_memory_operation(name, token, function),
            "atomic" => self.parse_atomic_operation(name, token, function),

            // function values, slices, and dynamic values
            "function" => self.parse_function_operation(name, token, function),
            "slice" => self.parse_slice_operation(name, token, function),
            "dynamic" => self.parse_dynamic_operation(name, token, function),

            // execution contexts
            "context" => self.parse_context_operation(name, token, function),

            // new and destruction
            "new" => self.parse_new(name, token, function),
            "release" | "free" | "drop" => self.parse_reference_operation(name, token, function),

            // collector protocol
            "barrier" => self.parse_reference_operation(name, token, function),

            // calls
            "call" | "invoke" | "tail" => self.parse_call_operation(name, token, function),

            // control flow
            "jump" | "branch" | "switch" | "await" | "yield" | "return" | "trap"
            | "unreachable" | "poll" | "breakpoint" => {
                self.parse_control_operation(name, token, function)
            }

            // panic and unwind
            "panic" | "unwind" => self.parse_control_operation(name, token, function),

            // runtime checks and profile instrumentation
            "check" => self.parse_check_operation(name, token, function),
            "profile" => self.parse_profile_operation(name, token, function),

            _ => Err(ParseError::new("unknown bytecode operation", token.span)),
        }
    }

    /// Parse destination operands from the leading result layout.
    pub(super) fn parse_definitions(&mut self, opcode: Opcode) -> ParseResult<Vec<RegisterSpan>> {
        let layout = opcode
            .layout()
            .ok_or_else(|| ParseError::new("unknown opcode layout", self.previous().span))?;
        let result_operands = layout
            .operands()
            .iter()
            .take_while(|operand| matches!(operand, Operand::Result | Operand::ResultRange))
            .copied()
            .collect::<Vec<_>>();
        let has_inputs = result_operands.len() < layout.operands().len();
        let mut definitions = Vec::with_capacity(result_operands.len());

        // parse each physical destination in encoded order
        for (index, operand) in result_operands.iter().copied().enumerate() {
            let definition = if operand == Operand::ResultRange && self.eat_name_if("_") {
                RegisterSpan::empty()
            } else if operand == Operand::Result {
                RegisterSpan::new(self.parse_register()?, 1)
            } else {
                self.parse_register_span()?
            };
            definitions.push(definition);

            // separate destinations from each other and from the first input
            if index + 1 < result_operands.len() || has_inputs {
                self.eat_token(TokenType::Comma)?;
            }
        }

        Ok(definitions)
    }

    /// Parse one exact number of physical destinations before opcode selection.
    pub(super) fn parse_results(
        &mut self,
        count: usize,
        has_inputs: bool,
    ) -> ParseResult<Vec<RegisterSpan>> {
        let mut results = Vec::with_capacity(count);

        // parse each destination in source order
        for index in 0..count {
            results.push(self.parse_register_span()?);
            if index + 1 < count || has_inputs {
                self.eat_token(TokenType::Comma)?;
            }
        }

        Ok(results)
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
            .strip_prefix('b')
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

    /// Parse one object-local allocation site id.
    pub(super) fn parse_allocation_id(&mut self) -> ParseResult<u32> {
        let token = self.eat_token(TokenType::Identifier)?;
        let index = self
            .text(token)
            .strip_prefix('a')
            .and_then(|index| index.parse().ok())
            .ok_or_else(|| ParseError::new("expected allocation site id", token.span))?;

        Ok(index)
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
