use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    FunctionId, FunctionTypeId, Opcode, RegisterId, RegisterRange, Symbol, SymbolTag, ValueType,
};

use super::instruction::InstructionFormatter;

/// One decoded bytecode call target.
enum CallTarget {
    /// One directly linked function.
    Direct {
        /// The linked function name.
        name: String,
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
    /// One dynamic receiver and slot.
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
    /// Return the exact called function type.
    const fn function_type(&self) -> FunctionTypeId {
        match self {
            Self::Direct { function_type, .. }
            | Self::Function { function_type, .. }
            | Self::FunctionPointer { function_type, .. }
            | Self::Virtual { function_type, .. }
            | Self::Dynamic { function_type, .. } => *function_type,
        }
    }

    /// Return whether this target uses a dispatch slot.
    const fn is_dispatched(&self) -> bool {
        matches!(self, Self::Virtual { .. } | Self::Dynamic { .. })
    }
}

impl InstructionFormatter<'_, '_, '_> {
    /// Format one direct or dispatched call.
    pub(super) fn format_call(&mut self, opcode: Opcode) -> FormatResult<()> {
        let is_tail = matches!(
            opcode,
            Opcode::TAIL_CALL
                | Opcode::TAIL_CALL_INDIRECT
                | Opcode::TAIL_CALL_FUNCTION_POINTER
                | Opcode::TAIL_CALL_VIRTUAL
                | Opcode::TAIL_CALL_DYNAMIC
        );
        let is_invoke = matches!(
            opcode,
            Opcode::INVOKE
                | Opcode::INVOKE_INDIRECT
                | Opcode::INVOKE_FUNCTION_POINTER
                | Opcode::INVOKE_VIRTUAL
                | Opcode::INVOKE_DYNAMIC
        );

        // decode the result range before the target that determines its types
        let results = if is_tail {
            None
        } else {
            let (start, word_count) = self.register_range_id()?;

            Some(RegisterRange::new(start, word_count))
        };
        let target = self.call_target(opcode)?;
        let function_type_id = target.function_type();
        let function_type = self
            .formatter
            .context()
            .object
            .function_type(function_type_id)
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "call references a missing function type",
            })?;
        let value_types = self.formatter.context().object.value_types();

        // write typed results followed by the canonical call operation
        if let Some(results) = results {
            self.write_results(
                results.start,
                results.word_count,
                function_type.results(value_types),
            )?;
            if results.word_count > 0 {
                write!(self.formatter, [space(), token("="), space()])?;
            }
        }
        let name = self.fixed_name(opcode)?;
        self.write_text(name)?;
        write!(self.formatter, [space()])?;
        self.write_call_target(&target)?;

        // write every packed argument as one logical value
        let arguments = self.typed_register_ids(function_type.parameters(value_types))?;
        self.write_token("(")?;
        for (index, argument) in arguments.into_iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), soft_line_break_or_space()])?;
            }
            self.write_register(argument)?;
        }
        self.write_token(")")?;
        if target.is_dispatched() {
            let name = self
                .formatter
                .context()
                .function_type_name(function_type_id)?
                .to_string();
            write!(self.formatter, [token(":"), space()])?;
            self.write_text(&name)?;
        }

        // write explicit normal and unwind edges as one breakable continuation
        if is_invoke {
            let normal = self.branch()?;
            let unwind = self.branch()?;
            self.continuation()?;
            write!(self.formatter, [token("=>"), space()])?;
            self.write_label(normal)?;
            write!(self.formatter, [space(), token("|"), space()])?;
            self.write_label(unwind)?;
        }

        Ok(())
    }

    /// Decode one call target and its exact function type.
    fn call_target(&mut self, opcode: Opcode) -> FormatResult<CallTarget> {
        // direct function
        if matches!(opcode, Opcode::CALL | Opcode::INVOKE | Opcode::TAIL_CALL) {
            return self.direct_call_target();
        }

        // open function value or pointer
        if matches!(
            opcode,
            Opcode::CALL_INDIRECT
                | Opcode::INVOKE_INDIRECT
                | Opcode::TAIL_CALL_INDIRECT
                | Opcode::CALL_FUNCTION_POINTER
                | Opcode::INVOKE_FUNCTION_POINTER
                | Opcode::TAIL_CALL_FUNCTION_POINTER
        ) {
            return self.indirect_call_target(opcode);
        }

        // virtual or dynamic dispatch
        self.dispatch_call_target(opcode)
    }

    /// Decode one directly linked call target.
    fn direct_call_target(&mut self) -> FormatResult<CallTarget> {
        let (name, symbol) = self.symbol_with_target()?;
        let function_type = self.direct_function_type(symbol)?;

        Ok(CallTarget::Direct {
            name,
            function_type,
        })
    }

    /// Decode one function value or function pointer call target.
    fn indirect_call_target(&mut self, opcode: Opcode) -> FormatResult<CallTarget> {
        let (_, symbol) = self.symbol_with_target()?;
        if symbol.tag != SymbolTag::FUNCTION_TYPE {
            return Err(FormatError::SyntaxError {
                message: "open call does not reference a function type",
            });
        }
        let function_type = FunctionTypeId(symbol.index);

        // function value target
        if matches!(
            opcode,
            Opcode::CALL_INDIRECT | Opcode::INVOKE_INDIRECT | Opcode::TAIL_CALL_INDIRECT
        ) {
            let (function, word_count) = self.register_range_id()?;
            if word_count != ValueType::function(function_type).word_count() {
                return Err(FormatError::SyntaxError {
                    message: "indirect call target has an invalid register width",
                });
            }

            return Ok(CallTarget::Function {
                function,
                function_type,
            });
        }

        // bare function pointer target
        let function = self.register_id()?;

        Ok(CallTarget::FunctionPointer {
            function,
            function_type,
        })
    }

    /// Decode one virtual or dynamic dispatch target.
    fn dispatch_call_target(&mut self, opcode: Opcode) -> FormatResult<CallTarget> {
        let (_, symbol) = self.symbol_with_target()?;
        if symbol.tag != SymbolTag::FUNCTION_TYPE {
            return Err(FormatError::SyntaxError {
                message: "dispatch call does not reference a function type",
            });
        }
        let function_type = FunctionTypeId(symbol.index);

        // virtual receiver
        if matches!(
            opcode,
            Opcode::CALL_VIRTUAL | Opcode::INVOKE_VIRTUAL | Opcode::TAIL_CALL_VIRTUAL
        ) {
            let receiver = self.register_id()?;
            let slot = self.u16()?;

            return Ok(CallTarget::Virtual {
                receiver,
                slot,
                function_type,
            });
        }

        // dynamic receiver
        let (dynamic, word_count) = self.register_range_id()?;
        if word_count != 2 {
            return Err(FormatError::SyntaxError {
                message: "dynamic call target has an invalid register width",
            });
        }
        let slot = self.u16()?;

        Ok(CallTarget::Dynamic {
            dynamic,
            slot,
            function_type,
        })
    }

    /// Write one decoded call target.
    fn write_call_target(&mut self, target: &CallTarget) -> FormatResult<()> {
        match target {
            CallTarget::Direct { name, .. } => self.write_text(name),
            CallTarget::Function { function, .. }
            | CallTarget::FunctionPointer { function, .. } => self.write_register(*function),
            CallTarget::Virtual { receiver, slot, .. } => {
                let slot = slot.to_string();
                self.write_register(*receiver)?;
                write!(
                    self.formatter,
                    [token(","), space(), token("slot"), space()]
                )?;
                self.write_text(&slot)
            }
            CallTarget::Dynamic { dynamic, slot, .. } => {
                let slot = slot.to_string();
                self.write_register(*dynamic)?;
                write!(
                    self.formatter,
                    [token(","), space(), token("slot"), space()]
                )?;
                self.write_text(&slot)?;

                Ok(())
            }
        }
    }

    /// Return the function type selected by one direct function symbol.
    fn direct_function_type(&self, symbol: Symbol) -> FormatResult<FunctionTypeId> {
        if symbol.tag != SymbolTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "direct call does not reference a function",
            });
        }

        self.formatter
            .context()
            .object
            .function(FunctionId(symbol.index))
            .map(|function| function.function_type)
            .ok_or(FormatError::SyntaxError {
                message: "direct call references a missing function",
            })
    }
}
