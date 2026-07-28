use destack_bytecode::{Instruction, Opcode};
use destack_program::{TypeId, Waiter, Word};

use crate::diagnostic::{Error, Result};
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one waiter operation.
    pub(crate) fn execute_waiter(&mut self, instruction: Instruction<'_>) -> Result<()> {
        match instruction.opcode() {
            Opcode::WAITER_QUEUE => self.execute_waiter_queue(instruction),
            Opcode::WAITER_CANCEL => self.execute_waiter_cancel(instruction),
            _ => unreachable!("waiter dispatch selects one waiter opcode"),
        }
    }

    /// Queue one runtime-owned asynchronous waiter.
    fn execute_waiter_queue(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let destination = operands.register()?;
        let waiter = operands.register()?;
        let ty = TypeId(operands.u32()?);
        let value = operands.span()?;
        let frame = self.frame();
        let waiter = Waiter::from_bits(self.read(waiter.0).bits());
        let words = self
            .machine
            .stack
            .words(frame.range(value), value.word_count as usize);
        let value = self
            .machine
            .program
            .value(ty, words)
            .map_err(Error::program)?;

        let is_settled = self
            .activation
            .runtime
            .queue_waiter(waiter, value)
            .map_err(Error::program)?;
        self.write(destination.0, Word::boolean(is_settled));

        Ok(())
    }

    /// Cancel one runtime-owned asynchronous waiter.
    fn execute_waiter_cancel(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let destination = operands.register()?;
        let waiter = operands.register()?;
        let waiter = Waiter::from_bits(self.read(waiter.0).bits());

        let is_settled = self
            .activation
            .runtime
            .cancel_waiter(waiter)
            .map_err(Error::program)?;
        self.write(destination.0, Word::boolean(is_settled));

        Ok(())
    }
}
