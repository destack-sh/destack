use destack_bytecode::{Instruction, Opcode};
use destack_program::TypeId;

use crate::diagnostic::{Error, Panic, Result, Trap};
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Begin unwinding one language panic.
    pub(crate) fn execute_panic(&mut self, instruction: Instruction<'_>) -> Result<()> {
        if self.panic.is_some() {
            return Err(Error::trap(Trap::Abort));
        }
        if let Some(function) = self.destructor() {
            return Err(Error::invalid_destructor(function));
        }
        let panic = match instruction.opcode() {
            Opcode::PANIC => Panic::empty(),
            Opcode::PANIC_VALUE => {
                let mut operands = instruction.operands();
                let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
                let range = operands.range().map_err(|_| self.invalid_instruction())?;
                let start = self.frame().range(range);
                let words = self.machine.stack.words(start, range.word_count as usize);

                Panic::new(ty, words)
            }
            _ => unreachable!("panic dispatch selects one panic opcode"),
        };
        let panic = self.locate(Error::panic(panic));
        self.panic = Some(panic);

        self.unwind()
    }

    /// Continue one pending panic beyond the active cleanup frame.
    pub(crate) fn execute_unwind_resume(&mut self) -> Result<()> {
        if self.panic.is_none() {
            return Err(self.invalid_instruction());
        }

        self.unwind()
    }

    /// Unwind frames until cleanup or the host boundary is reached.
    fn unwind(&mut self) -> Result<()> {
        loop {
            let Some(frame) = self.machine.frames.pop() else {
                unreachable!("panic unwinding requires an active frame");
            };
            self.machine.stack.truncate(frame.stack_offset);

            // report the panic after the entry frame leaves the machine
            let Some(caller) = self.machine.frames.last().copied() else {
                let Some(error) = self.panic.take() else {
                    unreachable!("panic unwinding requires a retained payload");
                };

                return Err(error);
            };

            // enter the nearest explicit unwind cleanup
            if let Some(unwind_state) = caller.unwind_state {
                let point = self
                    .machine
                    .program
                    .frame_point(unwind_state)
                    .ok_or_else(Error::invalid_continuation)?;
                let code_offset = self.code_offset(point)?;
                self.frame_mut().code_offset = code_offset;
                self.frame_mut().resume();

                return Ok(());
            }
        }
    }
}
