use tspp_bytecode::{Instruction, Opcode, Scalar};
use tspp_program::{Event, FrameEvent, LayoutShape, Runtime, TypeId, Word};

use crate::diagnostic::{Error, ExecutionResult, Panic, Result, Trap};
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
                let message = self.panic_message(ty, &words)?;

                (Panic::new(ty, words).message(message), Some(ty))
            }
            _ => unreachable!("panic dispatch selects one panic opcode"),
        };
        let point = self.point(self.frame(), self.pc)?;
        self.observe(Event::Panic { point, ty })?;

        let panic = self.locate(Error::panic(panic));
        self.panic = Some(panic);

        self.unwind()
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
