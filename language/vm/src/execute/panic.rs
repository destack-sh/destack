use tspp_bytecode::{CodeOffset, Instruction, Opcode, Scalar};
use tspp_program::{Event, FrameEvent, LayoutShape, Outcome, Runtime, TypeId, Word};

use crate::diagnostic::{Error, ExecutionResult, Panic, Result, Trap};
use crate::machine::{Activation, Return, Unwinding};

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Record one language panic as the active unwind.
    pub(crate) fn execute_panic(
        &mut self,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        if self.unwinding.is_some() {
            return Err(Error::trap(Trap::Abort).into());
        }

        // read the payload and render its message
        let (panic, ty) = if instruction.opcode() == Opcode::PANIC_VALUE {
            let mut operands = self.operands(instruction);
            let ty = TypeId(operands.u32()?);
            let range = operands.span()?;
            let start = self.frame().range(range);
            let words = self.fiber.stack.words(start, range.word_count as usize);
            let message = self.panic_message(ty, &words)?;

            (Panic::new(ty, words).message(message), Some(ty))
        } else {
            (Panic::empty(), None)
        };

        // observe the panic before its cleanups run
        let point = self.point(self.frame(), self.pc)?;
        self.observe(Event::Panic { point, ty })?;
        let panic = self.locate(Error::panic(panic));
        self.unwinding = Some(Unwinding::Panic(panic));

        Ok(())
    }

    /// Render the text of a string payload before unwinding.
    fn panic_message(&self, ty: TypeId, words: &[Word]) -> Result<Option<String>> {
        // render only references to the known string representation
        let program = &self.machine.program;
        let Some(string) = program.known().string else {
            return Ok(None);
        };
        let layout = program
            .layout(ty)
            .ok_or_else(|| self.invalid_instruction())?;
        let LayoutShape::Reference(reference) = layout.shape else {
            return Ok(None);
        };
        if reference.pointee != string.ty {
            return Ok(None);
        }

        // read the code unit slice address and count words
        let base = self.activation.memory.base_address();
        let object = words.first().ok_or_else(|| self.invalid_instruction())?;
        let slice = base + object.bits() as usize + string.units_offset as usize;
        let word_bytes = program.pointer_bytes() as usize;
        let units = base + self.load(slice, Scalar::Uint64).bits() as usize;
        let count = self.load(slice + word_bytes, Scalar::Uint64).bits() as usize;

        // decode the UTF-16 code units, marking lone surrogates
        let units =
            (0..count).map(|index| self.load(units + 2 * index, Scalar::Uint16).bits() as u16);
        let message = char::decode_utf16(units)
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();

        Ok(Some(message))
    }

    /// Continue one pending panic or cancellation beyond the active cleanup frame.
    pub(crate) fn execute_unwind_resume(
        &mut self,
    ) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
        if self.unwinding.is_none() {
            return Err(self.invalid_instruction().into());
        }

        self.unwind()
    }

    /// Unwind a cancellation from one parked call, entering its own cleanup first.
    pub(crate) fn cancel(
        &mut self,
        unwind: Option<CodeOffset>,
    ) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
        self.unwinding = Some(Unwinding::Cancel);

        // enter the park call's cleanup, else unwind its frame
        self.activate();
        match unwind {
            Some(unwind) => {
                self.jump(unwind);
                self.save_position();

                Ok(None)
            }
            None => {
                let outcome = self.unwind()?;
                if outcome.is_none() {
                    self.save_position();
                }

                Ok(outcome)
            }
        }
    }

    /// Unwind frames until cleanup or the host boundary is reached.
    fn unwind(&mut self) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
        loop {
            let Some(frame) = self.fiber.frames.pop() else {
                unreachable!("panic unwinding requires an active frame");
            };
            self.fiber.stack.truncate(frame.byte_offset());
            self.observe(Event::Frame {
                event: FrameEvent::Exit,
                function: frame.function,
            })?;

            // report a panic or complete a cancellation after the entry frame leaves
            let Some(_caller) = self.fiber.frames.last().copied() else {
                return match self.unwinding.take() {
                    Some(Unwinding::Panic(error)) => Err(error.into()),
                    Some(Unwinding::Cancel) => Ok(Some(Outcome::Cancelled)),
                    None => unreachable!("unwinding requires a retained reason"),
                };
            };
            self.activate();

            // enter the nearest explicit unwind cleanup
            let unwind = match frame.return_to {
                Return::Call { unwind, .. } => unwind,
                Return::Exit { .. } | Return::Drop { .. } | Return::Release(_) => None,
            };
            if let Some(unwind) = unwind {
                self.jump(unwind);

                return Ok(None);
            }
        }
    }
}
