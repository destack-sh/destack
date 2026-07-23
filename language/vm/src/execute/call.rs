use std::ptr;

use destack_bytecode::{CodeOffset, Instruction, Opcode, Operands};
use destack_program::{DynamicTableId, FunctionId, VirtualTableId, Word};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Return};

/// One executable bytecode callee.
struct Callee {
    /// The linked function.
    function: FunctionId,
    /// The hidden closure environment when present.
    environment: Option<Word>,
}

impl Activation<'_, '_> {
    /// Execute one direct, indirect, or dynamic bytecode call.
    pub(crate) fn execute_call(
        &mut self,
        instruction: Instruction<'_>,
        instruction_offset: CodeOffset,
    ) -> Result<()> {
        let opcode = instruction.opcode();
        let mut operands = instruction.operands();

        // replace the current frame directly for tail calls
        if Self::is_tail_call(opcode) {
            let callee = self.callee(opcode, &mut operands)?;
            let arguments = operands.range().map_err(|_| self.invalid_instruction())?;

            return self.tail_call(callee.function, arguments, callee.environment);
        }

        // decode the result range before the selected callee form
        let results = operands.range().map_err(|_| self.invalid_instruction())?;
        let callee = self.callee(opcode, &mut operands)?;
        let arguments = operands.range().map_err(|_| self.invalid_instruction())?;
        let (normal, unwind) = if Self::is_invoke(opcode) {
            let normal = operands.i32().map_err(|_| self.invalid_instruction())?;
            let unwind = operands.i32().map_err(|_| self.invalid_instruction())?;

            (Some(normal), Some(unwind))
        } else {
            (None, None)
        };

        self.call(
            callee.function,
            arguments,
            callee.environment,
            Return::Values(results),
            instruction_offset,
            normal,
            unwind,
        )
    }

    /// Return one value range from the active frame.
    pub(crate) fn execute_return(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<Option<Vec<Word>>> {
        let mut operands = instruction.operands();
        let results = operands.range().map_err(|_| self.invalid_instruction())?;

        self.return_values(results)
    }

    /// Decode the linked callee and optional hidden environment.
    fn callee(&self, opcode: Opcode, operands: &mut Operands<'_>) -> Result<Callee> {
        match opcode {
            // direct function
            Opcode::CALL | Opcode::INVOKE | Opcode::TAIL_CALL => {
                let function = operands.u32().map_err(|_| self.invalid_instruction())?;

                Ok(Callee {
                    function: FunctionId(function),
                    environment: None,
                })
            }

            // indirect function value or pointer
            Opcode::CALL_INDIRECT | Opcode::INVOKE_INDIRECT | Opcode::TAIL_CALL_INDIRECT => {
                let value = operands.range().map_err(|_| self.invalid_instruction())?;
                let environment = match value.word_count {
                    1 => None,
                    2 => Some(self.read(value.start.0 + 1)),
                    _ => return Err(self.invalid_instruction()),
                };

                Ok(Callee {
                    function: self.function_id(self.read(value.start.0))?,
                    environment,
                })
            }

            // virtual table id stored in the concrete object
            Opcode::CALL_VIRTUAL | Opcode::INVOKE_VIRTUAL | Opcode::TAIL_CALL_VIRTUAL => {
                let receiver = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let reference = operands
                    .reference()
                    .map_err(|_| self.invalid_instruction())?;
                let dispatch_offset = operands.u32().map_err(|_| self.invalid_instruction())?;
                let slot = operands.u16().map_err(|_| self.invalid_instruction())?;
                let edge = self.read_reference_edge(receiver, reference)?;
                let address = self.call.memory.native_address(edge) + dispatch_offset as usize;

                // SAFETY: linked virtual calls use the dispatch field from the receiver layout
                let table = unsafe { ptr::read_unaligned(address as *const u32) };
                let function = self
                    .machine
                    .program
                    .virtual_method(VirtualTableId(table), u32::from(slot))
                    .ok_or_else(|| self.invalid_instruction())?;

                Ok(Callee {
                    function,
                    environment: None,
                })
            }

            // dynamic dispatch table carried by the erased value
            Opcode::CALL_DYNAMIC | Opcode::INVOKE_DYNAMIC | Opcode::TAIL_CALL_DYNAMIC => {
                let dynamic = operands.range().map_err(|_| self.invalid_instruction())?;
                let slot = operands.u16().map_err(|_| self.invalid_instruction())?;
                if dynamic.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                let table = DynamicTableId::from(self.read(dynamic.start.0 + 1));
                let function = self
                    .machine
                    .program
                    .dynamic_entry(table, u32::from(slot))
                    .and_then(|entry| entry.function_value())
                    .ok_or_else(|| self.invalid_instruction())?;

                Ok(Callee {
                    function,
                    environment: None,
                })
            }

            // unsupported opcode
            _ => Err(Error::unsupported_opcode(opcode.code())),
        }
    }

    /// Return whether one call carries normal and unwind branches.
    const fn is_invoke(opcode: Opcode) -> bool {
        matches!(
            opcode,
            Opcode::INVOKE
                | Opcode::INVOKE_INDIRECT
                | Opcode::INVOKE_VIRTUAL
                | Opcode::INVOKE_DYNAMIC
        )
    }

    /// Return whether one call replaces the active frame.
    const fn is_tail_call(opcode: Opcode) -> bool {
        matches!(
            opcode,
            Opcode::TAIL_CALL
                | Opcode::TAIL_CALL_INDIRECT
                | Opcode::TAIL_CALL_VIRTUAL
                | Opcode::TAIL_CALL_DYNAMIC
        )
    }
}
