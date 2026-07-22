use crate::{
    Comparison, InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId,
    RegisterRange, Scalar, ScalarCheck, Symbol, Token, TokenType, Trap, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one control operation.
    pub(super) fn parse_control_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            // control flow
            "jump" => self.parse_jump(results, function),
            "branch" => self.parse_conditional_branch(token, results, function),
            "switch" => self.parse_switch(token, results, function),
            "yield" => self.parse_yield(token, results, function),
            "return" => self.parse_return(token, results, function),
            "trap" => self.parse_trap(results, function),
            "unreachable" => self.parse_empty_control(Opcode::UNREACHABLE, results, function),

            // panic and unwind
            "panic" => self.parse_panic(results, function),
            "unwind.resume" => self.parse_empty_control(Opcode::UNWIND_RESUME, results, function),

            // collector protocol
            "safepoint" => self.parse_empty_control(Opcode::SAFEPOINT, results, function),

            // debug control
            "breakpoint" => self.parse_empty_control(Opcode::BREAKPOINT, results, function),
            _ => Err(ParseError::new("invalid control operation", token.span)),
        }
    }

    /// Parse one unconditional jump.
    fn parse_jump(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let mut instruction = InstructionBuilder::new(Opcode::JUMP);
        instruction.branch(self.parse_label()?);

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one boolean conditional branch.
    fn parse_conditional_branch(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let condition = self.parse_register()?;
        if !function.has_type(condition, ValueType::scalar(Scalar::Boolean)) {
            return Err(ParseError::new(
                "branch condition is not boolean",
                token.span,
            ));
        }

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

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one integer switch.
    fn parse_switch(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register()?;
        let is_integer = function
            .value_type(value)
            .and_then(ValueType::scalar_type)
            .is_some_and(Scalar::is_integer);
        if !is_integer {
            return Err(ParseError::new(
                "switch value is not a scalar integer",
                token.span,
            ));
        }

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

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one suspension and its resume and unwind destinations.
    fn parse_yield(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let resume_parameters = function.resume_parameters.clone();

        // parse yielded logical values
        let values = if self.peek_is(TokenType::FatArrow) {
            Vec::new()
        } else {
            self.parse_registers()?
        };
        let types = values
            .iter()
            .map(|value| function.value_type(*value))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| ParseError::new("yield reads an uninitialized value", token.span))?;
        let values = RegisterRange::pack(&values, &types)
            .ok_or_else(|| ParseError::new("yield values are not contiguous", token.span))?;
        let ty = match types.as_slice() {
            [] => ValueType::void(),
            [ty] => *ty,
            _ => {
                return Err(ParseError::new(
                    "yield requires at most one logical value",
                    token.span,
                ));
            }
        };

        // parse resume and unwind destinations
        self.eat_token(TokenType::FatArrow)?;
        let resume_label = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
        let unwind_label = self.parse_label()?;

        // encode the complete suspension
        let mut instruction = InstructionBuilder::new(Opcode::YIELD);
        instruction.range(values);
        instruction.value_type(ty);
        instruction.branch(resume_label);
        instruction.branch(unwind_label);

        function.emit(instruction, results, &resume_parameters, self.empty_span())
    }

    /// Parse one function return.
    fn parse_return(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let values = if self.peek_is(TokenType::CloseBrace) || self.is_label() {
            Vec::new()
        } else if self.is_register() {
            self.parse_registers()?
        } else {
            Vec::new()
        };
        if !function.values_match(&values, &function.results) {
            return Err(ParseError::new(
                "return values do not match the function signature",
                token.span,
            ));
        }
        let values = RegisterRange::pack(&values, &function.results)
            .ok_or_else(|| ParseError::new("return values are not contiguous", token.span))?;

        // encode the contiguous result window
        let mut instruction = InstructionBuilder::new(Opcode::RETURN);
        instruction.range(values);

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one explicit trap.
    fn parse_trap(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let reason = self.eat_token(TokenType::Identifier)?;
        let reason = Trap::from_name(self.text(reason))
            .ok_or_else(|| ParseError::new("unknown trap reason", reason.span))?;
        let mut instruction = InstructionBuilder::new(Opcode::TRAP);
        instruction.u16(reason as u16);

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one panic with an optional value.
    fn parse_panic(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let instruction = if self.is_register() {
            let values = self.parse_registers()?;
            let types = values
                .iter()
                .map(|value| function.value_type(*value))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    ParseError::new("panic reads an uninitialized value", self.empty_span())
                })?;
            let values = RegisterRange::pack(&values, &types).ok_or_else(|| {
                ParseError::new("panic value is not contiguous", self.empty_span())
            })?;
            self.eat_token(TokenType::Colon)?;
            let ty = self.parse_type_name()?;
            let mut instruction = InstructionBuilder::new(Opcode::PANIC_VALUE);
            instruction.symbol(Symbol::ty(ty.0));
            instruction.range(values);

            instruction
        } else {
            InstructionBuilder::new(Opcode::PANIC)
        };

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one control operation without operands.
    fn parse_empty_control(
        &mut self,
        opcode: Opcode,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let instruction = InstructionBuilder::new(opcode);

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one runtime check instruction.
    pub(super) fn parse_check_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let name = name
            .strip_prefix("check.")
            .ok_or_else(|| ParseError::new("expected check operation", token.span))?;
        let mut instruction = if name == "null" {
            self.parse_null_check(token, function)?
        } else if name == "type" || name == "subtype" {
            self.parse_type_check(name, token, function)?
        } else {
            self.parse_scalar_check(name, token, function)?
        };

        // parse the failure destination shared by every check
        self.eat_name("else")?;
        instruction.branch(self.parse_label()?);

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one null check before its failure destination.
    fn parse_null_check(
        &mut self,
        token: Token,
        function: &FunctionParser,
    ) -> ParseResult<InstructionBuilder> {
        let value = self.parse_register()?;
        let value_type = function.value_type(value).ok_or_else(|| {
            ParseError::new("null check reads an uninitialized value", token.span)
        })?;
        if !value_type.is_pointer() && !value_type.is_initialized_reference() {
            return Err(ParseError::new(
                "null check requires a pointer or reference",
                token.span,
            ));
        }

        let mut instruction = InstructionBuilder::new(Opcode::CHECK_NULL);
        instruction.register(value);

        Ok(instruction)
    }

    /// Parse one exact type or subtype check before its failure destination.
    fn parse_type_check(
        &mut self,
        name: &str,
        token: Token,
        function: &FunctionParser,
    ) -> ParseResult<InstructionBuilder> {
        let value = self.parse_register()?;
        if !function.has_type(value, ValueType::type_id()) {
            return Err(ParseError::new(
                "runtime type check requires a type id",
                token.span,
            ));
        }

        // parse the expected linked type
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type_name()?;

        // encode the selected relation
        let opcode = if name == "type" {
            Opcode::CHECK_EXACT_TYPE
        } else {
            Opcode::CHECK_SUBTYPE
        };
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(value);
        instruction.symbol(Symbol::ty(ty.0));

        Ok(instruction)
    }

    /// Parse one scalar check before its failure destination.
    fn parse_scalar_check(
        &mut self,
        name: &str,
        token: Token,
        function: &FunctionParser,
    ) -> ParseResult<InstructionBuilder> {
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
        let value_type = ValueType::scalar(scalar);
        if !function.has_type(value, value_type) {
            return Err(ParseError::new(
                "check input does not match its scalar type",
                token.span,
            ));
        }
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
                let bound = self.parse_check_value(
                    value_type,
                    "bounds check inputs do not match",
                    token,
                    function,
                )?;
                instruction.register(bound);
            }
            ScalarCheck::Range => {
                let start = self.parse_check_value(
                    value_type,
                    "range check inputs do not match",
                    token,
                    function,
                )?;
                let length = self.parse_check_value(
                    value_type,
                    "range check inputs do not match",
                    token,
                    function,
                )?;
                instruction.register(start);
                instruction.register(length);
            }
            ScalarCheck::AddOverflow
            | ScalarCheck::SubtractOverflow
            | ScalarCheck::MultiplyOverflow => {
                let right = self.parse_check_value(
                    value_type,
                    "overflow check inputs do not match",
                    token,
                    function,
                )?;
                instruction.register(right);
            }
            ScalarCheck::Nonzero => {}
        }

        Ok(instruction)
    }

    /// Parse one comma-prefixed scalar check operand.
    fn parse_check_value(
        &mut self,
        ty: ValueType,
        message: &'static str,
        token: Token,
        function: &FunctionParser,
    ) -> ParseResult<RegisterId> {
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;
        if !function.has_type(value, ty) {
            return Err(ParseError::new(message, token.span));
        }

        Ok(value)
    }

    /// Parse one fused scalar branch instruction.
    pub(super) fn parse_branch(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
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
        let value_type = ValueType::scalar(scalar);
        if !function.has_type(left, value_type) || !function.has_type(right, value_type) {
            return Err(ParseError::new(
                "branch inputs do not match its scalar type",
                token.span,
            ));
        }

        // parse both branch destinations
        self.eat_token(TokenType::FatArrow)?;
        let success = self.parse_label()?;
        self.eat_token(TokenType::Comma)?;
        let failure = self.parse_label()?;

        // encode the fused comparison and branch
        let opcode = Opcode::branch(comparison, scalar)
            .ok_or_else(|| ParseError::new("invalid branch operand", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(left);
        instruction.register(right);
        instruction.branch(success);
        instruction.branch(failure);

        function.emit(instruction, results, &[], self.empty_span())
    }
}
