use crate::{
    FunctionId, FunctionTypeId, Opcode, ParseError, ParseResult, Parser, RegisterId, RegisterRange,
    Symbol, Token, TokenType, ValueType,
};

use super::builder::InstructionBuilder;
use super::function::FunctionBuilder;

/// One parsed bytecode call target.
#[derive(Clone, Copy, Debug)]
enum CallTarget {
    /// One direct function symbol.
    Direct {
        /// The called function.
        function: FunctionId,
        /// The called function type.
        function_type: FunctionTypeId,
    },
    /// One function value.
    Function {
        /// The function value.
        function: RegisterId,
        /// The called function type.
        function_type: FunctionTypeId,
    },
    /// One bare function pointer.
    FunctionPointer {
        /// The function pointer.
        function: RegisterId,
        /// The called function type.
        function_type: FunctionTypeId,
    },
    /// One virtual receiver and slot.
    Virtual {
        /// The receiver reference.
        receiver: RegisterId,
        /// The virtual method slot.
        slot: u16,
        /// The called function type.
        function_type: FunctionTypeId,
    },
    /// One dynamic value and slot.
    Dynamic {
        /// The dynamic value.
        dynamic: RegisterId,
        /// The dynamic method slot.
        slot: u16,
        /// The called function type.
        function_type: FunctionTypeId,
    },
}

impl CallTarget {
    /// Return the called function type.
    const fn function_type(self) -> FunctionTypeId {
        match self {
            Self::Direct { function_type, .. }
            | Self::Function { function_type, .. }
            | Self::FunctionPointer { function_type, .. }
            | Self::Virtual { function_type, .. }
            | Self::Dynamic { function_type, .. } => function_type,
        }
    }

    /// Encode this target into one call instruction.
    fn encode(self, instruction: &mut InstructionBuilder) {
        match self {
            Self::Direct { function, .. } => {
                instruction.symbol(Symbol::function(function.0));
            }
            Self::Function {
                function,
                function_type,
            } => {
                instruction.symbol(Symbol::function_type(function_type.0));
                instruction.range(RegisterRange::new(function, 2));
            }
            Self::FunctionPointer {
                function,
                function_type,
            } => {
                instruction.symbol(Symbol::function_type(function_type.0));
                instruction.register(function);
            }
            Self::Virtual {
                receiver,
                slot,
                function_type,
            } => {
                instruction.symbol(Symbol::function_type(function_type.0));
                instruction.register(receiver);
                instruction.u16(slot);
            }
            Self::Dynamic {
                dynamic,
                slot,
                function_type,
            } => {
                instruction.symbol(Symbol::function_type(function_type.0));
                instruction.range(RegisterRange::new(dynamic, 2));
                instruction.u16(slot);
            }
        }
    }

    /// Select the exact opcode for this target and control-flow form.
    const fn opcode(self, is_invoke: bool, is_tail: bool) -> Opcode {
        match (self, is_invoke, is_tail) {
            (Self::Direct { .. }, false, false) => Opcode::CALL,
            (Self::Direct { .. }, true, false) => Opcode::INVOKE,
            (Self::Direct { .. }, false, true) => Opcode::TAIL_CALL,
            (Self::Function { .. }, false, false) => Opcode::CALL_INDIRECT,
            (Self::Function { .. }, true, false) => Opcode::INVOKE_INDIRECT,
            (Self::Function { .. }, false, true) => Opcode::TAIL_CALL_INDIRECT,
            (Self::FunctionPointer { .. }, false, false) => Opcode::CALL_FUNCTION_POINTER,
            (Self::FunctionPointer { .. }, true, false) => Opcode::INVOKE_FUNCTION_POINTER,
            (Self::FunctionPointer { .. }, false, true) => Opcode::TAIL_CALL_FUNCTION_POINTER,
            (Self::Virtual { .. }, false, false) => Opcode::CALL_VIRTUAL,
            (Self::Virtual { .. }, true, false) => Opcode::INVOKE_VIRTUAL,
            (Self::Virtual { .. }, false, true) => Opcode::TAIL_CALL_VIRTUAL,
            (Self::Dynamic { .. }, false, false) => Opcode::CALL_DYNAMIC,
            (Self::Dynamic { .. }, true, false) => Opcode::INVOKE_DYNAMIC,
            (Self::Dynamic { .. }, false, true) => Opcode::TAIL_CALL_DYNAMIC,
            _ => Opcode::INVALID,
        }
    }
}

impl Parser<'_> {
    /// Parse one direct, indirect, virtual, or dynamic call.
    pub(super) fn parse_call_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        let is_tail = name.starts_with("tail.");
        let is_invoke = name.starts_with("invoke");
        if is_tail && !results.is_empty() {
            return Err(ParseError::new(
                "tail calls do not assign results",
                token.span,
            ));
        }

        let (target, arguments) = self.parse_call_target(name, function)?;
        let opcode = target.opcode(is_invoke, is_tail);
        if opcode == Opcode::INVALID {
            return Err(ParseError::new("invalid call operation", token.span));
        }
        let function_type = target.function_type();
        let types = self.symbols.function_type_definitions[function_type.index()].clone();

        // match packed arguments against the function type
        if !function.values_match(&arguments, &types.parameters) {
            return Err(ParseError::new(
                "call arguments do not match its function type",
                token.span,
            ));
        }
        let arguments = RegisterRange::pack(&arguments, &types.parameters)
            .ok_or_else(|| ParseError::new("call arguments are not contiguous", token.span))?;

        // match tail results against the current function
        let do_tail_results_match = !is_tail || function.results == types.results;
        if !do_tail_results_match {
            return Err(ParseError::new(
                "tail call results do not match its function",
                token.span,
            ));
        }

        // encode the call in stable operand order
        let mut instruction = InstructionBuilder::new(opcode);
        target.encode(&mut instruction);
        instruction.range(arguments);

        // append normal and unwind destinations for invokes
        if is_invoke {
            self.eat_token(TokenType::FatArrow)?;
            instruction.branch(self.parse_label()?);
            self.eat_token(TokenType::Pipe)?;
            instruction.branch(self.parse_label()?);
        }

        let result_types = if is_tail { &[][..] } else { &types.results };

        function.emit(instruction, results, result_types, self.empty_span())
    }

    /// Parse one call target and its logical arguments.
    fn parse_call_target(
        &mut self,
        name: &str,
        function: &FunctionBuilder,
    ) -> ParseResult<(CallTarget, Vec<RegisterId>)> {
        // direct target
        if matches!(name, "call" | "invoke" | "tail.call") {
            return self.parse_direct_call_target();
        }

        // function value or pointer target
        if name.ends_with("indirect") {
            return self.parse_indirect_call_target(function);
        }

        // virtual or dynamic dispatch target
        self.parse_dispatch_call_target(name, function)
    }

    /// Parse one directly linked call target.
    fn parse_direct_call_target(&mut self) -> ParseResult<(CallTarget, Vec<RegisterId>)> {
        let symbol = self.eat_token(TokenType::Identifier)?;
        let function = self.function_symbol(self.text(symbol), symbol.span)?;
        let arguments = self.parse_argument_registers()?;
        let function_type = self.symbols.function_declarations[function.index()].function_type;
        let target = CallTarget::Direct {
            function,
            function_type,
        };

        Ok((target, arguments))
    }

    /// Parse one function value or function pointer call target.
    fn parse_indirect_call_target(
        &mut self,
        function: &FunctionBuilder,
    ) -> ParseResult<(CallTarget, Vec<RegisterId>)> {
        let register = self.parse_register()?;
        let arguments = self.parse_argument_registers()?;
        let target_type = function.value_type(register).ok_or_else(|| {
            ParseError::new(
                "indirect call reads an uninitialized value",
                self.previous().span,
            )
        })?;
        let function_type = target_type.function_type().ok_or_else(|| {
            ParseError::new(
                "indirect call requires a function value",
                self.previous().span,
            )
        })?;

        // preserve the physical representation selected by the register type
        let target = if target_type.is_function_pointer() {
            CallTarget::FunctionPointer {
                function: register,
                function_type,
            }
        } else {
            CallTarget::Function {
                function: register,
                function_type,
            }
        };

        Ok((target, arguments))
    }

    /// Parse one virtual or dynamic dispatch target.
    fn parse_dispatch_call_target(
        &mut self,
        name: &str,
        function: &FunctionBuilder,
    ) -> ParseResult<(CallTarget, Vec<RegisterId>)> {
        let receiver = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        self.eat_name("slot")?;
        let slot = self.parse_u16()?;
        let arguments = self.parse_argument_registers()?;
        self.eat_token(TokenType::Colon)?;
        let function_type = self.parse_named_function_type()?;
        let receiver_type = function.value_type(receiver);

        // dynamic dispatch
        let target = if name.ends_with("dynamic") {
            if !receiver_type.is_some_and(ValueType::is_dynamic) {
                return Err(ParseError::new(
                    "dynamic call requires a dynamic value",
                    self.previous().span,
                ));
            }
            CallTarget::Dynamic {
                dynamic: receiver,
                slot,
                function_type,
            }
        }
        // virtual dispatch
        else {
            if !receiver_type.is_some_and(ValueType::is_initialized_reference) {
                return Err(ParseError::new(
                    "virtual call requires an initialized reference",
                    self.previous().span,
                ));
            }
            CallTarget::Virtual {
                receiver,
                slot,
                function_type,
            }
        };

        Ok((target, arguments))
    }

    /// Parse one named function type.
    fn parse_named_function_type(&mut self) -> ParseResult<FunctionTypeId> {
        let name = self.eat_token(TokenType::Identifier)?;

        self.symbols
            .function_types
            .get(self.text(name))
            .copied()
            .ok_or_else(|| ParseError::new("unknown function type", name.span))
    }

    /// Parse one parenthesized logical argument list.
    fn parse_argument_registers(&mut self) -> ParseResult<Vec<RegisterId>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut registers = Vec::new();
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            registers.push(self.parse_register()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }

        Ok(registers)
    }
}
