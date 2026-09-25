use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{Opcode, ReferenceType, RegisterId, RegisterSpan, RelocationTag, ValueType};

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
            let (start, word_count) = self.register_span_id()?;

            Some(RegisterSpan::new(start, word_count))
        };
        let target = self.call_target(opcode)?;

        // write the operation and physical results
        let name = self.opcode_name(opcode)?;
        self.write_opcode(name)?;
        if let Some(results) = results {
            self.write_span(results)?;
            self.write_comma()?;
        }
        self.write_call_target(&target)?;

        // write the packed physical argument span
        let (start, word_count) = self.register_span_id()?;
        let arguments = RegisterSpan::new(start, word_count);
        self.write_token("(")?;
        if arguments.word_count > 0 {
            self.write_span(arguments)?;
        }
        self.write_token(")")?;

        // write explicit normal and unwind successors
        if is_invoke {
            let normal = self.branch()?;
            let unwind = self.branch()?;
            self.write_break()?;
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
        let (name, relocation) = self.relocation_with_text()?;
        if relocation.tag != RelocationTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "direct call does not reference a function",
            });
        }

        Ok(CallTarget::Direct { name })
    }

    /// Decode one function value or function pointer call target.
    fn indirect_call_target(&mut self) -> FormatResult<CallTarget> {
        let (start, word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(start, word_count);
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
            return Ok(CallTarget::Virtual {
                receiver,
                reference,
                dispatch_offset,
                slot,
            });
        }

        // dynamic receiver
        let (receiver, word_count) = self.register_span_id()?;
        if word_count != 2 {
            return Err(FormatError::SyntaxError {
                message: "dynamic call target has an invalid register width",
            });
        }
        let slot = self.u16()?;

        let receiver = RegisterSpan::new(receiver, word_count);

        Ok(CallTarget::Dynamic { receiver, slot })
    }

    /// Write one decoded call target.
    fn write_call_target(&mut self, target: &CallTarget) -> FormatResult<()> {
        match target {
            CallTarget::Direct { name } => self.write_text(name),
            CallTarget::Indirect { value } => self.write_span(*value),
            CallTarget::Virtual {
                receiver,
                reference,
                dispatch_offset,
                slot,
            } => {
                let dispatch_offset = dispatch_offset.to_string();
                let slot = slot.to_string();
                self.write_register(*receiver)?;
                self.write_token(":")?;
                write!(self.formatter, [space()])?;
                write!(
                    self.formatter,
                    [&ValueType::reference(reference.kind(), reference.storage())]
                )?;
                self.write_token("[")?;
                self.write_text(&dispatch_offset)?;
                self.write_comma()?;
                self.write_text(&slot)?;
                self.write_token("]")
            }
            CallTarget::Dynamic { receiver, slot } => {
                let slot = slot.to_string();
                self.write_span(*receiver)?;
                self.write_token("[")?;
                self.write_text(&slot)?;
                self.write_token("]")?;

                Ok(())
            }
        }
    }
}
