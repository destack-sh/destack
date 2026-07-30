use crate::{
    Comparison, InstructionBuilder, Label, Opcode, ParseError, ParseResult, Parser, RegisterId,
    RegisterSpan, RelocationTag, Scalar, ScalarCheck, Token, TokenType, Trap,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one control operation.
    pub(super) fn parse_control_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "jump" => Opcode::JUMP,
            "branch" => Opcode::BRANCH,
            "switch" => Opcode::SWITCH,
            "await" => Opcode::AWAIT,
            "yield" => Opcode::YIELD,
            "return" => Opcode::RETURN,
            "trap" => Opcode::TRAP,
            "unreachable" => Opcode::UNREACHABLE,
            "panic" => Opcode::PANIC,
            "unwind.resume" => Opcode::UNWIND_RESUME,
            "breakpoint" => Opcode::BREAKPOINT,
            "poll" => Opcode::POLL,
            _ => return Err(ParseError::new("invalid control operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match name {
            // control flow
            "jump" => self.parse_jump(&results, function),
            "branch" => self.parse_conditional_branch(&results, function),
            "switch" => self.parse_switch(token, &results, function),
            "await" => self.parse_await(&results, function),
            "yield" => self.parse_yield(&results, function),
            "return" => self.parse_return(&results, function),
            "trap" => self.parse_trap(&results, function),
            "unreachable" => self.parse_empty_control(Opcode::UNREACHABLE, &results, function),

            // panic and unwind
            "panic" => self.parse_panic(&results, function),
            "unwind.resume" => self.parse_empty_control(Opcode::UNWIND_RESUME, &results, function),

            // runtime control
            "poll" => self.parse_empty_control(Opcode::POLL, &results, function),

            // debug control
            "breakpoint" => self.parse_empty_control(Opcode::BREAKPOINT, &results, function),
            _ => Err(ParseError::new("invalid control operation", token.span)),
        }
    }

    /// Parse one unconditional jump.
    fn parse_jump(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let mut instruction = InstructionBuilder::new(Opcode::JUMP);
        instruction.branch(self.parse_label()?);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one boolean conditional branch.
    fn parse_conditional_branch(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let condition = self.parse_register()?;

        // parse both destinations
        self.eat_token(TokenType::Comma)?;
        let then_label = self.parse_label()?;
        self.eat_token(TokenType::Comma)?;
        let else_label = self.parse_label()?;

        // encode condition and destinations
        let mut instruction = InstructionBuilder::new(Opcode::BRANCH);
        instruction.register(condition);
        instruction.branch(then_label);
        instruction.branch(else_label);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one integer switch.
    fn parse_switch(
        &mut self,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register()?;
        // parse cases up to the mandatory fallback
        self.eat_token(TokenType::OpenBrace)?;
        let mut cases = Vec::new();
        while !self.peek_name("default") {
            let case = self.parse_u64()?;
            self.eat_token(TokenType::FatArrow)?;
            cases.push((case, self.parse_label()?));
            self.eat_token(TokenType::Comma)?;
        }
        self.eat_name("default")?;
        self.eat_token(TokenType::FatArrow)?;
        let fallback = self.parse_label()?;
        self.eat_token(TokenType::CloseBrace)?;

        // encode cases in source order
        let mut instruction = InstructionBuilder::new(Opcode::SWITCH);
        let case_count = u16::try_from(cases.len())
            .map_err(|_| ParseError::new("too many switch cases", token.span))?;
        instruction.register(value);
        instruction.u16(case_count);
        for (case, label) in cases {
            instruction.u64(case);
            instruction.branch(label);
        }
        instruction.branch(fallback);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one asynchronous suspension.
    fn parse_await(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let park = self.parse_function_id()?;
        self.eat_token(TokenType::Comma)?;
        let awaitable = self.parse_register_span()?;
        let (resume, cancel, unwind) = self.parse_await_targets()?;

        // encode the selected park implementation and consumed awaitable
        let mut instruction = InstructionBuilder::new(Opcode::AWAIT);
        instruction.relocation(RelocationTag::FUNCTION, park.0);
        instruction.span(awaitable);
        instruction.branch(resume);
        instruction.branch(cancel);
        instruction.branch(unwind);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one generator suspension.
    fn parse_yield(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register_span()?;
        let (resume, complete, unwind) = self.parse_yield_targets()?;
        let mut instruction = InstructionBuilder::new(Opcode::YIELD);
        instruction.span(value);
        instruction.branch(resume);
        instruction.branch(complete);
        instruction.branch(unwind);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse resume, cancellation, and unwind destinations for one await.
    fn parse_await_targets(&mut self) -> ParseResult<(Label, Label, Label)> {
        self.eat_token(TokenType::FatArrow)?;
        let resume = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
        let cancel = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
        let unwind = self.parse_label()?;

        Ok((resume, cancel, unwind))
    }

    /// Parse resume, completion, and unwind destinations for one yield.
    fn parse_yield_targets(&mut self) -> ParseResult<(Label, Label, Label)> {
        self.eat_token(TokenType::FatArrow)?;
        let resume = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
        let complete = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
        let unwind = self.parse_label()?;

        Ok((resume, complete, unwind))
    }

    /// Parse one function return.
    fn parse_return(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let values = if self.peek_is(TokenType::CloseBrace) || self.is_label() {
            RegisterSpan::new(RegisterId(0), 0)
        } else if self.is_register() {
            self.parse_register_span()?
        } else {
            RegisterSpan::new(RegisterId(0), 0)
        };

        // encode the contiguous result window
        let mut instruction = InstructionBuilder::new(Opcode::RETURN);
        instruction.span(values);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one explicit trap.
    fn parse_trap(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let reason = self.eat_token(TokenType::Identifier)?;
        let reason = Trap::from_name(self.text(reason))
            .ok_or_else(|| ParseError::new("unknown trap reason", reason.span))?;
        let mut instruction = InstructionBuilder::new(Opcode::TRAP);
        instruction.u16(reason as u16);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one panic with an optional value.
    fn parse_panic(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let instruction = if self.is_register() {
            let values = self.parse_register_span()?;
            self.eat_token(TokenType::Comma)?;
            let ty = self.parse_type_id()?;
            let mut instruction = InstructionBuilder::new(Opcode::PANIC_VALUE);
            instruction.relocation(RelocationTag::TYPE, ty.0);
            instruction.span(values);

            instruction
        } else {
            InstructionBuilder::new(Opcode::PANIC)
        };

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one control operation without operands.
    fn parse_empty_control(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let instruction = InstructionBuilder::new(opcode);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one runtime check instruction.
    pub(super) fn parse_check_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let name = name
            .strip_prefix("check.")
            .ok_or_else(|| ParseError::new("expected check operation", token.span))?;
        let mut instruction = if name == "null" {
            self.parse_null_check()?
        } else if name == "type" || name == "subtype" {
            self.parse_type_check(name)?
        } else {
            self.parse_scalar_check(name, token)?
        };

        // parse the failure destination shared by every check
        self.eat_name("else")?;
        instruction.branch(self.parse_label()?);

        let results = self.parse_definitions(instruction.opcode)?;

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one null check before its failure destination.
    fn parse_null_check(&mut self) -> ParseResult<InstructionBuilder> {
        let value = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(Opcode::CHECK_NULL);
        instruction.register(value);

        Ok(instruction)
    }

    /// Parse one exact type or subtype check before its failure destination.
    fn parse_type_check(&mut self, name: &str) -> ParseResult<InstructionBuilder> {
        let value = self.parse_register()?;

        // parse the expected linked type
        self.eat_token(TokenType::Comma)?;
        let ty = self.parse_type_id()?;

        // encode the selected relation
        let opcode = if name == "type" {
            Opcode::CHECK_EXACT_TYPE
        } else {
            Opcode::CHECK_SUBTYPE
        };
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(value);
        instruction.relocation(RelocationTag::TYPE, ty.0);

        Ok(instruction)
    }

    /// Parse one scalar check before its failure destination.
    fn parse_scalar_check(&mut self, name: &str, token: Token) -> ParseResult<InstructionBuilder> {
        // parse the operation and scalar suffix
        let (operation_name, scalar_name) = name
            .rsplit_once('.')
            .ok_or_else(|| ParseError::new("check operation has no scalar type", token.span))?;
        let operation = ScalarCheck::from_name(operation_name)
            .ok_or_else(|| ParseError::new("unknown check operation", token.span))?;
        let scalar = Scalar::from_name(scalar_name)
            .filter(|scalar| operation.supports(*scalar))
            .ok_or_else(|| ParseError::new("invalid check scalar type", token.span))?;

        // parse the primary scalar input
        let value = self.parse_register()?;
        let opcode = Opcode::check(operation, scalar)
            .ok_or_else(|| ParseError::new("invalid check operand", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(value);

        // parse operation-specific bounds
        match operation {
            ScalarCheck::Shift => {
                self.eat_token(TokenType::Comma)?;
                instruction.u16(self.parse_u16()?);
            }
            ScalarCheck::Narrow => {
                self.eat_token(TokenType::Arrow)?;
                instruction.scalar(self.parse_scalar_name()?);
            }
            ScalarCheck::Bounds => {
                let bound = self.parse_check_value()?;
                instruction.register(bound);
            }
            ScalarCheck::Range => {
                let start = self.parse_check_value()?;
                let length = self.parse_check_value()?;
                instruction.register(start);
                instruction.register(length);
            }
            ScalarCheck::AddOverflow
            | ScalarCheck::SubtractOverflow
            | ScalarCheck::MultiplyOverflow => {
                let right = self.parse_check_value()?;
                instruction.register(right);
            }
            ScalarCheck::Nonzero => {}
        }

        Ok(instruction)
    }

    /// Parse one comma-prefixed scalar check operand.
    fn parse_check_value(&mut self) -> ParseResult<RegisterId> {
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;

        Ok(value)
    }

    /// Parse one fused scalar branch instruction.
    pub(super) fn parse_branch(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // resolve the exact comparison and scalar suffix
        let mut components = name.split('.');
        let branch = components.next();
        let comparison_name = components.next();
        let scalar_name = components.next();
        if branch != Some("branch") || components.next().is_some() {
            return Err(ParseError::new("invalid branch operation", token.span));
        }
        let comparison_name = comparison_name
            .ok_or_else(|| ParseError::new("branch has no comparison", token.span))?;
        let scalar = scalar_name
            .and_then(Scalar::from_name)
            .ok_or_else(|| ParseError::new("branch has no scalar type", token.span))?;
        let comparison = Comparison::from_name(comparison_name)
            .ok_or_else(|| ParseError::new("unknown branch comparison", token.span))?;

        // parse both scalar inputs
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        // parse both branch destinations
        self.eat_token(TokenType::FatArrow)?;
        let success = self.parse_label()?;
        self.eat_token(TokenType::Comma)?;
        let failure = self.parse_label()?;

        // encode the fused comparison and branch
        let opcode = Opcode::branch(comparison, scalar)
            .ok_or_else(|| ParseError::new("invalid branch operand", token.span))?;
        let results = self.parse_definitions(opcode)?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(left);
        instruction.register(right);
        instruction.branch(success);
        instruction.branch(failure);

        function.emit(instruction, &results, self.empty_span())
    }
}
