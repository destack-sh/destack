use crate::{
    Comparison, InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId,
    RegisterSpan, RelocationTag, Scalar, ScalarCheck, Token, TokenType, Trap,
};

use super::function::FunctionParser;

/// One operation-specific scalar check argument.
enum CheckArgument {
    /// No additional argument.
    None,
    /// One static bit width.
    Width(u16),
    /// One target scalar representation.
    Target(Scalar),
    /// One dynamic scalar bound.
    Register(RegisterId),
    /// One dynamic start and length.
    Range(RegisterId, RegisterId),
}

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
        self.eat_token(TokenType::FatArrow)?;
        let then_label = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
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
        instruction.register(value);
        instruction
            .switch(&cases)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        instruction.branch(fallback);

        function.emit(instruction, results, self.empty_span())
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
        let mut instruction = if name == "nullish" {
            self.parse_nullish_check()?
        } else if name == "type" || name == "subtype" {
            self.parse_type_check(name)?
        } else {
            return Err(ParseError::new("unknown check operation", token.span));
        };

        // parse the failure destination shared by every check
        self.eat_token(TokenType::Pipe)?;
        instruction.branch(self.parse_label()?);

        let results = self.parse_definitions(instruction.opcode)?;

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one nullish check before its failure destination.
    fn parse_nullish_check(&mut self) -> ParseResult<InstructionBuilder> {
        let value = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(Opcode::CHECK_NULLISH);
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

    /// Parse one scalar runtime check.
    pub(super) fn parse_scalar_check(
        &mut self,
        scalar: Scalar,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let name = name
            .strip_prefix("check.")
            .ok_or_else(|| ParseError::new("expected scalar check", token.span))?;
        let (name, target) = match name.strip_prefix("narrow.") {
            Some(target) => ("narrow", Scalar::from_name(target)),
            None => (name, None),
        };
        let operation = ScalarCheck::from_name(name)
            .ok_or_else(|| ParseError::new("unknown check operation", token.span))?;

        // parse the primary scalar input and operation-specific argument
        let value = self.parse_register()?;
        let argument = match operation {
            ScalarCheck::Shift => {
                self.eat_token(TokenType::Comma)?;
                CheckArgument::Width(self.parse_u16()?)
            }
            ScalarCheck::Narrow => CheckArgument::Target(
                target
                    .ok_or_else(|| ParseError::new("narrow check requires a target", token.span))?,
            ),
            ScalarCheck::Bounds => {
                let bound = self.parse_check_value()?;
                CheckArgument::Register(bound)
            }
            ScalarCheck::Range => {
                let start = self.parse_check_value()?;
                let length = self.parse_check_value()?;
                CheckArgument::Range(start, length)
            }
            ScalarCheck::AddOverflow
            | ScalarCheck::SubtractOverflow
            | ScalarCheck::MultiplyOverflow => {
                let right = self.parse_check_value()?;
                CheckArgument::Register(right)
            }
            ScalarCheck::Nonzero => CheckArgument::None,
        };

        let opcode = Opcode::check(operation, scalar)
            .ok_or_else(|| ParseError::new("invalid check operand", token.span))?;

        // encode the complete typed check
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(value);
        match argument {
            CheckArgument::None => {}
            CheckArgument::Width(width) => instruction.u16(width),
            CheckArgument::Target(target) => instruction.scalar(target),
            CheckArgument::Register(register) => instruction.register(register),
            CheckArgument::Range(start, length) => {
                instruction.register(start);
                instruction.register(length);
            }
        }

        // parse the common failure destination
        self.eat_token(TokenType::Pipe)?;
        instruction.branch(self.parse_label()?);
        let results = self.parse_definitions(instruction.opcode)?;

        function.emit(instruction, &results, self.empty_span())
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
        scalar: Scalar,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let comparison_name = name
            .strip_prefix("branch.")
            .ok_or_else(|| ParseError::new("invalid branch operation", token.span))?;
        let comparison = Comparison::from_name(comparison_name)
            .ok_or_else(|| ParseError::new("unknown branch comparison", token.span))?;

        // parse both scalar inputs
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        // parse both branch destinations
        self.eat_token(TokenType::FatArrow)?;
        let success = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
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
