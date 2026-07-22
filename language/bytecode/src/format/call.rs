use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, RegisterId, RegisterRange, SymbolTag};

use super::instruction::InstructionFormatter;

/// One decoded bytecode call target.
enum CallTarget {
    /// One directly linked function.
    Direct {
        /// The linked function name.
        name: String,
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

impl InstructionFormatter<'_, '_, '_> {
    /// Format one direct or dispatched call.
    pub(super) fn format_call(&mut self, opcode: Opcode) -> FormatResult<()> {
        let is_tail = matches!(
            opcode,
            Opcode::TAIL_CALL
                | Opcode::TAIL_CALL_INDIRECT
                | Opcode::TAIL_CALL_VIRTUAL
                | Opcode::TAIL_CALL_DYNAMIC
        );
        let is_invoke = matches!(
            opcode,
            Opcode::INVOKE
                | Opcode::INVOKE_INDIRECT
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
        let result_types = match results {
            Some(results) => self.formatter.context().register_types(results)?,
            None => Vec::new(),
        };

        // write typed results followed by the canonical call operation
        if let Some(results) = results {
            self.write_results(results.start, results.word_count, &result_types)?;
            if results.word_count > 0 {
                write!(self.formatter, [space(), token("="), space()])?;
            }
        }
        let name = self.opcode_name(opcode)?;
        self.write_text(name)?;
        write!(self.formatter, [space()])?;
        self.write_call_target(&target)?;

        // write every packed argument as one logical value
        let arguments = self.register_value_ids()?;
        self.write_token("(")?;
        for (index, argument) in arguments.into_iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), soft_line_break_or_space()])?;
            }
            self.write_register(argument)?;
        }
        self.write_token(")")?;

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

    /// Decode one call target.
    fn call_target(&mut self, opcode: Opcode) -> FormatResult<CallTarget> {
        // direct function
        if matches!(opcode, Opcode::CALL | Opcode::INVOKE | Opcode::TAIL_CALL) {
            return self.direct_call_target();
        }

        // open function value or pointer
        if matches!(
            opcode,
            Opcode::CALL_INDIRECT | Opcode::INVOKE_INDIRECT | Opcode::TAIL_CALL_INDIRECT
        ) {
            return self.indirect_call_target();
        }

        // virtual or dynamic dispatch
        self.dispatch_call_target(opcode)
    }

    /// Decode one directly linked call target.
    fn direct_call_target(&mut self) -> FormatResult<CallTarget> {
        let (name, symbol) = self.symbol_with_target()?;
        if symbol.tag != SymbolTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "direct call does not reference a function",
            });
        }

        Ok(CallTarget::Direct { name })
    }

    /// Decode one function value or function pointer call target.
    fn indirect_call_target(&mut self) -> FormatResult<CallTarget> {
        let (start, word_count) = self.register_range_id()?;
        let value = RegisterRange::new(start, word_count);
        let value_type = self.formatter.context().register_type(start)?;
        if (!value_type.is_function() && !value_type.is_function_pointer())
            || value_type.word_count() != word_count
        {
            return Err(FormatError::SyntaxError {
                message: "indirect call target is not callable",
            });
        }

        Ok(CallTarget::Indirect { value })
    }

    /// Decode one virtual or dynamic dispatch target.
    fn dispatch_call_target(&mut self, opcode: Opcode) -> FormatResult<CallTarget> {
        // virtual receiver
        if matches!(
            opcode,
            Opcode::CALL_VIRTUAL | Opcode::INVOKE_VIRTUAL | Opcode::TAIL_CALL_VIRTUAL
        ) {
            let receiver = self.register_id()?;
            let reference = self.reference()?;
            let dispatch_offset = self.u32()?;
            let slot = self.u16()?;
            let receiver_type = self.formatter.context().register_type(receiver)?;
            if receiver_type.reference_type() != Some(reference) {
                return Err(FormatError::SyntaxError {
                    message: "virtual call receiver does not match its reference operand",
                });
            }

            return Ok(CallTarget::Virtual {
                receiver,
                dispatch_offset,
                slot,
            });
        }

        // dynamic receiver
        let (receiver, word_count) = self.register_range_id()?;
        if word_count != 2 {
            return Err(FormatError::SyntaxError {
                message: "dynamic call target has an invalid register width",
            });
        }
        let slot = self.u16()?;

        Ok(CallTarget::Dynamic { receiver, slot })
    }

    /// Write one decoded call target.
    fn write_call_target(&mut self, target: &CallTarget) -> FormatResult<()> {
        match target {
            CallTarget::Direct { name, .. } => self.write_text(name),
            CallTarget::Indirect { value, .. } => self.write_register(value.start),
            CallTarget::Virtual {
                receiver,
                dispatch_offset,
                slot,
                ..
            } => {
                let dispatch_offset = dispatch_offset.to_string();
                let slot = slot.to_string();
                self.write_register(*receiver)?;
                write!(
                    self.formatter,
                    [token(","), space(), token("dispatch"), space()]
                )?;
                self.write_text(&dispatch_offset)?;
                write!(
                    self.formatter,
                    [token(","), space(), token("slot"), space()]
                )?;
                self.write_text(&slot)
            }
            CallTarget::Dynamic { receiver, slot, .. } => {
                let slot = slot.to_string();
                self.write_register(*receiver)?;
                write!(
                    self.formatter,
                    [token(","), space(), token("slot"), space()]
                )?;
                self.write_text(&slot)?;

                Ok(())
            }
        }
    }
}
