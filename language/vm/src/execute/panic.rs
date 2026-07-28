use destack_bytecode::{Instruction, Opcode};
use destack_program::{Runtime, Task, TaskOutcome, TypeId};

use crate::diagnostic::{Error, ExecutionError, ExecutionResult, Panic, Trap};
use crate::machine::{Activation, Return};

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Begin unwinding one language panic.
    pub(crate) fn execute_panic(
        &mut self,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        if self.panic.is_some() {
            return Err(Error::trap(Trap::Abort).into());
        }
        let panic = match instruction.opcode() {
            Opcode::PANIC => Panic::empty(),
            Opcode::PANIC_VALUE => {
                let mut operands = self.operands(instruction);
                let ty = TypeId(operands.u32()?);
                let range = operands.span()?;
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
    pub(crate) fn execute_unwind_resume(&mut self) -> ExecutionResult<(), R::Error> {
        if self.panic.is_none() {
            return Err(self.invalid_instruction().into());
        }

        self.unwind()
    }

    /// Unwind frames until cleanup or the host boundary is reached.
    fn unwind(&mut self) -> ExecutionResult<(), R::Error> {
        loop {
            let Some(frame) = self.machine.frames.pop() else {
                unreachable!("panic unwinding requires an active frame");
            };
            self.machine.stack.truncate(frame.byte_offset());

            // report the panic after the entry frame leaves the machine
            let Some(_caller) = self.machine.frames.last().copied() else {
                let Some(error) = self.panic.take() else {
                    unreachable!("panic unwinding requires a retained payload");
                };

                return Err(error.into());
            };
            self.activate();

            // settle task state before unwinding through its eager caller
            if let Return::Task { task_register, .. } = frame.return_to {
                let task = Task::from_bits(self.read(task_register).bits());
                self.activation
                    .runtime
                    .finish_task(task, TaskOutcome::Cancelled)
                    .map_err(ExecutionError::runtime)?;
            }

            // enter the nearest explicit unwind cleanup
            let unwind = match frame.return_to {
                Return::Call { unwind, .. } | Return::Continuation { unwind, .. } => unwind,
                Return::Exit { .. } | Return::Task { .. } | Return::Drop { .. } => None,
            };
            if let Some(unwind) = unwind {
                self.jump(unwind);

                return Ok(());
            }
        }
    }
}
