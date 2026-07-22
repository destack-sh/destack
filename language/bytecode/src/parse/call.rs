use crate::{
    FunctionId, InstructionBuilder, Opcode, ParseError, ParseResult, Parser, ReferenceType,
    RegisterId, RegisterRange, Symbol, Token, TokenType, ValueType,
};

use super::function::FunctionParser;

/// One parsed bytecode call target.
#[derive(Clone, Copy, Debug)]
enum CallTarget {
    /// One direct function symbol.
    Direct {
        /// The called function.
        function: FunctionId,
    },
    /// One indirect function value or pointer.
    Indirect {
        /// The callable value.
        value: RegisterRange,
    },
    /// One virtual receiver and slot.
    Virtual {
        /// The receiver reference.
        receiver: RegisterId,
        /// The receiver reference representation.
        reference: ReferenceType,
        /// The byte offset of the virtual table id in the receiver allocation.
        dispatch_offset: u32,
        /// The virtual method slot.
        slot: u16,
    },
    /// One dynamic receiver and slot.
    Dynamic {
        /// The receiver value.
        receiver: RegisterId,
        /// The dynamic method slot.
        slot: u16,
    },
}

impl CallTarget {
    /// Encode this target into one call instruction.
    fn encode(self, instruction: &mut InstructionBuilder) {
        match self {
            Self::Direct { function, .. } => {
                instruction.symbol(Symbol::function(function.0));
            }
            Self::Indirect { value } => {
                instruction.range(value);
            }
            Self::Virtual {
                receiver,
                reference,
                dispatch_offset,
                slot,
            } => {
                instruction.register(receiver);
                instruction.reference(reference.kind(), reference.space());
                instruction.u32(dispatch_offset);
                instruction.u16(slot);
            }
            Self::Dynamic { receiver, slot } => {
                instruction.range(RegisterRange::new(receiver, 2));
                instruction.u16(slot);
            }
        }
    }

    /// Select the exact opcode for this target and control-flow form.
    const fn opcode(self, is_invoke: bool, is_tail: bool) -> Option<Opcode> {
        match (self, is_invoke, is_tail) {
            (Self::Direct { .. }, false, false) => Some(Opcode::CALL),
            (Self::Direct { .. }, true, false) => Some(Opcode::INVOKE),
            (Self::Direct { .. }, false, true) => Some(Opcode::TAIL_CALL),
            (Self::Indirect { .. }, false, false) => Some(Opcode::CALL_INDIRECT),
            (Self::Indirect { .. }, true, false) => Some(Opcode::INVOKE_INDIRECT),
            (Self::Indirect { .. }, false, true) => Some(Opcode::TAIL_CALL_INDIRECT),
            (Self::Virtual { .. }, false, false) => Some(Opcode::CALL_VIRTUAL),
            (Self::Virtual { .. }, true, false) => Some(Opcode::INVOKE_VIRTUAL),
            (Self::Virtual { .. }, false, true) => Some(Opcode::TAIL_CALL_VIRTUAL),
            (Self::Dynamic { .. }, false, false) => Some(Opcode::CALL_DYNAMIC),
            (Self::Dynamic { .. }, true, false) => Some(Opcode::INVOKE_DYNAMIC),
            (Self::Dynamic { .. }, false, true) => Some(Opcode::TAIL_CALL_DYNAMIC),
            _ => None,
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
        result_types: &[ValueType],
        function: &mut FunctionParser,
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
        let opcode = target
            .opcode(is_invoke, is_tail)
            .ok_or_else(|| ParseError::new("invalid call operation", token.span))?;
        let argument_types = arguments
            .iter()
            .map(|register| function.value_type(*register))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| ParseError::new("call reads an uninitialized argument", token.span))?;
        let arguments = RegisterRange::pack(&arguments, &argument_types)
            .ok_or_else(|| ParseError::new("call arguments are not contiguous", token.span))?;

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

        let result_types = if is_tail { &[][..] } else { result_types };

        function.emit(instruction, results, result_types, self.empty_span())
    }

    /// Parse one call target and its logical arguments.
    fn parse_call_target(
        &mut self,
        name: &str,
        function: &FunctionParser,
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
        if self.symbols.function_declarations[function.index()]
            .environment
            .is_some()
        {
            return Err(ParseError::new(
                "direct call requires an environment-free function",
                symbol.span,
            ));
        }
        let arguments = self.parse_argument_registers()?;
        let target = CallTarget::Direct { function };

        Ok((target, arguments))
    }

    /// Parse one function value or function pointer call target.
    fn parse_indirect_call_target(
        &mut self,
        function: &FunctionParser,
    ) -> ParseResult<(CallTarget, Vec<RegisterId>)> {
        let register = self.parse_register()?;
        let arguments = self.parse_argument_registers()?;
        let target_type = function.value_type(register).ok_or_else(|| {
            ParseError::new(
                "indirect call reads an uninitialized value",
                self.previous().span,
            )
        })?;
        if !target_type.is_function() && !target_type.is_function_pointer() {
            return Err(ParseError::new(
                "indirect call requires a function value",
                self.previous().span,
            ));
        }

        let target = CallTarget::Indirect {
            value: RegisterRange::new(register, target_type.word_count()),
        };

        Ok((target, arguments))
    }

    /// Parse one virtual or dynamic dispatch target.
    fn parse_dispatch_call_target(
        &mut self,
        name: &str,
        function: &FunctionParser,
    ) -> ParseResult<(CallTarget, Vec<RegisterId>)> {
        let receiver = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let dispatch = if name.ends_with("virtual") {
            self.eat_name("dispatch")?;
            let dispatch = self.parse_u32()?;
            self.eat_token(TokenType::Comma)?;

            Some(dispatch)
        } else {
            None
        };
        self.eat_name("slot")?;
        let slot = self.parse_u16()?;
        let arguments = self.parse_argument_registers()?;
        let receiver_type = function.value_type(receiver);

        // dynamic dispatch
        let target = if name.ends_with("dynamic") {
            if !receiver_type.is_some_and(ValueType::is_dynamic) {
                return Err(ParseError::new(
                    "dynamic call requires a dynamic value",
                    self.previous().span,
                ));
            }
            CallTarget::Dynamic { receiver, slot }
        }
        // virtual dispatch
        else {
            if !receiver_type.is_some_and(ValueType::is_initialized_reference) {
                return Err(ParseError::new(
                    "virtual call requires an initialized reference",
                    self.previous().span,
                ));
            }
            let reference = receiver_type
                .and_then(ValueType::reference_type)
                .ok_or_else(|| {
                    ParseError::new("virtual call requires a reference", self.previous().span)
                })?;
            CallTarget::Virtual {
                receiver,
                reference,
                dispatch_offset: dispatch.ok_or_else(|| {
                    ParseError::new(
                        "virtual call requires a dispatch offset",
                        self.previous().span,
                    )
                })?,
                slot,
            }
        };

        Ok((target, arguments))
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
