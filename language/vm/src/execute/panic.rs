use tspp_bytecode::{Instruction, Opcode};
use tspp_program::{Event, FrameEvent, Runtime, TypeId};

use crate::diagnostic::{Error, ExecutionResult, Panic, Trap};
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
        let (panic, ty) = match instruction.opcode() {
            Opcode::PANIC => (Panic::empty(), None),
            Opcode::PANIC_VALUE => {
                let mut operands = self.operands(instruction);
                let ty = TypeId(operands.u32()?);
                let range = operands.span()?;
                let start = self.frame().range(range);
                let words = self.fiber.stack.words(start, range.word_count as usize);

                (Panic::new(ty, words), Some(ty))
            }
            _ => unreachable!("panic dispatch selects one panic opcode"),
        };
        let point = self.point(self.frame(), self.pc)?;
        self.observe(Event::Panic { point, ty })?;

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
            let Some(frame) = self.fiber.frames.pop() else {
                unreachable!("panic unwinding requires an active frame");
            };
            self.fiber.stack.truncate(frame.byte_offset());
            self.observe(Event::Frame {
                event: FrameEvent::Exit,
                function: frame.function,
            })?;

            // report the panic after the entry frame leaves the machine
            let Some(_caller) = self.fiber.frames.last().copied() else {
                let Some(error) = self.panic.take() else {
                    unreachable!("panic unwinding requires a retained payload");
                };

                return Err(error.into());
            };
            self.activate();

            // enter the nearest explicit unwind cleanup
            let unwind = match frame.return_to {
                Return::Call { unwind, .. } => unwind,
                Return::Exit { .. } | Return::Drop { .. } | Return::Release(_) => None,
            };
            if let Some(unwind) = unwind {
                self.jump(unwind);

                return Ok(());
            }
        }
    }
}
