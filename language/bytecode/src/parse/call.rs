use crate::{
    FunctionId, InstructionBuilder, Opcode, ParseError, ParseResult, Parser, ReferenceType,
    RegisterId, RegisterSpan, RelocationTag, Token, TokenType,
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
        value: RegisterSpan,
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
        receiver: RegisterSpan,
        /// The dynamic method slot.
        slot: u16,
    },
}

impl CallTarget {
    /// Encode this target into one call instruction.
    fn encode(self, instruction: &mut InstructionBuilder) {
        match self {
            Self::Direct { function, .. } => {
                instruction.relocation(RelocationTag::FUNCTION, function.0);
            }
            Self::Indirect { value } => {
                instruction.span(value);
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
                instruction.span(receiver);
                instruction.u16(slot);
            }
        }
    }
}

impl Parser<'_> {
    /// Parse one direct, indirect, virtual, or dynamic call.
    pub(super) fn parse_call_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let is_invoke = name.starts_with("invoke");
        let opcode = match name {
            "call" => Opcode::CALL,
            "call.indirect" => Opcode::CALL_INDIRECT,
            "call.virtual" => Opcode::CALL_VIRTUAL,
            "call.dynamic" => Opcode::CALL_DYNAMIC,
            "invoke" => Opcode::INVOKE,
            "invoke.indirect" => Opcode::INVOKE_INDIRECT,
            "invoke.virtual" => Opcode::INVOKE_VIRTUAL,
            "invoke.dynamic" => Opcode::INVOKE_DYNAMIC,
            "tail.call" => Opcode::TAIL_CALL,
            "tail.call.indirect" => Opcode::TAIL_CALL_INDIRECT,
            "tail.call.virtual" => Opcode::TAIL_CALL_VIRTUAL,
            "tail.call.dynamic" => Opcode::TAIL_CALL_DYNAMIC,
            _ => return Err(ParseError::new("invalid call operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        let (target, arguments) = self.parse_call_target(name)?;

        // encode the call in stable operand order
        let mut instruction = InstructionBuilder::new(opcode);
        target.encode(&mut instruction);
        instruction.span(arguments);

        // append normal and unwind destinations for invokes
        if is_invoke {
            self.eat_token(TokenType::FatArrow)?;
            instruction.branch(self.parse_label()?);
            self.eat_token(TokenType::Pipe)?;
            instruction.branch(self.parse_label()?);
        }

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one call target and its logical arguments.
    fn parse_call_target(&mut self, name: &str) -> ParseResult<(CallTarget, RegisterSpan)> {
        // direct target
        if matches!(name, "call" | "invoke" | "tail.call") {
            return self.parse_direct_call_target();
        }

        // function value or pointer target
        if name.ends_with("indirect") {
            return self.parse_indirect_call_target();
        }

        // virtual or dynamic dispatch target
        self.parse_dispatch_call_target(name)
    }

    /// Parse one directly linked call target.
    fn parse_direct_call_target(&mut self) -> ParseResult<(CallTarget, RegisterSpan)> {
        let function = self.parse_function_id()?;
        self.eat_token(TokenType::Comma)?;
        let arguments = self.parse_argument_span()?;
        let target = CallTarget::Direct { function };

        Ok((target, arguments))
    }

    /// Parse one function value or function pointer call target.
    fn parse_indirect_call_target(&mut self) -> ParseResult<(CallTarget, RegisterSpan)> {
        let value = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let arguments = self.parse_argument_span()?;
        let target = CallTarget::Indirect { value };

        Ok((target, arguments))
    }

    /// Parse one virtual or dynamic dispatch target.
    fn parse_dispatch_call_target(
        &mut self,
        name: &str,
    ) -> ParseResult<(CallTarget, RegisterSpan)> {
        // virtual dispatch
        if name.ends_with("virtual") {
            let receiver = self.parse_register()?;
            self.eat_token(TokenType::Comma)?;
            let reference = self
                .parse_value_type()?
                .reference_type()
                .ok_or_else(|| ParseError::new("expected reference type", self.previous().span))?;
            self.eat_token(TokenType::Comma)?;
            let dispatch_offset = self.parse_u32()?;
            self.eat_token(TokenType::Comma)?;
            let slot = self.parse_u16()?;
            self.eat_token(TokenType::Comma)?;
            let arguments = self.parse_argument_span()?;
            let target = CallTarget::Virtual {
                receiver,
                reference,
                dispatch_offset,
                slot,
            };

            return Ok((target, arguments));
        }

        // dynamic dispatch
        let receiver = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let slot = self.parse_u16()?;
        self.eat_token(TokenType::Comma)?;
        let arguments = self.parse_argument_span()?;
        let target = CallTarget::Dynamic { receiver, slot };

        Ok((target, arguments))
    }

    /// Parse one physical argument span.
    fn parse_argument_span(&mut self) -> ParseResult<RegisterSpan> {
        if self.eat_name_if("_") {
            Ok(RegisterSpan::new(RegisterId(0), 0))
        } else {
            self.parse_register_span()
        }
    }
}
