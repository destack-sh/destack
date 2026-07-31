use destack_bytecode::{CodeOffset, Instruction, Opcode};
use destack_program::{Continuation, ContinuationId, FrameImage, FramePoint, FunctionId, Runtime};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Return};

use super::suspension::SuspensionEdge;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one continuation operation.
    pub(crate) fn execute_continuation(
        &mut self,
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<()> {
        match instruction.opcode() {
            Opcode::CONTINUATION_NEW => self.execute_continuation_new(instruction),
            Opcode::CONTINUATION_DESTROY => self.execute_continuation_destroy(pc, instruction),
            Opcode::CONTINUATION_RESUME | Opcode::CONTINUATION_COMPLETE => {
                self.execute_continuation_transfer(pc, instruction)
            }
            _ => unreachable!("continuation dispatch selects one continuation opcode"),
        }
    }

    /// Create one ready continuation over captured arguments.
    fn execute_continuation_new(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let function = FunctionId(operands.u32()?);
        let arguments = operands.span()?;
        let is_coroutine = self
            .machine
            .program
            .function(function)
            .and_then(|function| function.coroutine())
            .is_some();
        if !is_coroutine {
            return Err(self.invalid_instruction());
        }
        let frame = self.frame();
        let arguments = self
            .machine
            .stack
            .words(frame.range(arguments), arguments.word_count as usize);
        self.save_position();
        let continuation = self.machine.create_continuation(function, &arguments)?;
        self.activate();
        let id = self.continuations.insert(continuation);

        self.write(target.0, id.into_word());

        Ok(())
    }

    /// Destroy one ready or suspended continuation.
    fn execute_continuation_destroy(
        &mut self,
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let continuation = operands.register()?;
        let id = ContinuationId::from_word(self.read(continuation.0));
        let continuation = self
            .continuations
            .take(id)
            .ok_or_else(|| self.invalid_instruction())?;

        self.destroy_continuation(pc, continuation)
    }

    /// Destroy every owned value retained by one continuation.
    fn destroy_continuation(&mut self, pc: CodeOffset, continuation: Continuation) -> Result<()> {
        let first_frame = self.machine.frames.len();
        let byte_len = self.machine.stack.byte_len();
        let owner = self.frame();
        self.save_position();

        // materialize and release the consumed continuation as one transition
        let (retained_state, frame_count, destructors) =
            self.machine
                .consume_continuation(continuation, |machine, continuation| {
                    let owner_state = machine.frame_state_at(owner, pc)?;
                    let return_to = Return::Drop {
                        pc,
                        caller_state: owner_state,
                        frame_count: 0,
                    };
                    let retained_state = continuation
                        .innermost()
                        .map(FrameImage::state)
                        .ok_or_else(Error::invalid_instruction)?;
                    let frame_count = u16::try_from(continuation.frames().len())
                        .map_err(|_| Error::invalid_instruction())?;
                    machine.materialize_continuation(continuation, return_to)?;
                    let destructors = match machine.frame_destructors(first_frame, retained_state) {
                        Ok(destructors) => destructors,
                        Err(error) => {
                            machine.frames.truncate(first_frame);
                            machine.stack.truncate(byte_len);

                            return Err(error);
                        }
                    };

                    Ok((retained_state, frame_count, destructors))
                })?;

        // release retained frames without destructible values immediately
        if destructors.is_empty() {
            self.machine.release_frames(first_frame)?;
            self.activate();

            return Ok(());
        }

        // push in acquisition order so execution destroys in reverse order
        for (index, (function, byte_offset)) in destructors.into_iter().enumerate() {
            let parent = self.frame();
            let parent_pc = parent.pc;
            let parent_state = if index == 0 {
                retained_state
            } else {
                self.machine.frame_state_at(parent, parent_pc)?
            };
            let released_frames = if index == 0 { frame_count } else { 0 };
            if let Err(error) = self.call_destructor(
                function,
                byte_offset,
                parent_pc,
                parent_state,
                released_frames,
            ) {
                self.machine.frames.truncate(first_frame);
                self.machine.stack.truncate(byte_len);

                return Err(error);
            }
        }

        Ok(())
    }

    /// Drive one ready or suspended continuation.
    fn execute_continuation_transfer(
        &mut self,
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let yielded_registers = operands.span()?;
        let continuation_register = operands.register()?.0;
        let returned_registers = operands.span()?;
        let continuation = operands.register()?;
        let value = operands.span()?;
        let yielded = operands.i32()?;
        let returned = operands.i32()?;
        let unwind = operands.i32()?;
        let frame = self.frame();
        let id = ContinuationId::from_word(self.read(continuation.0));

        let edge = match instruction.opcode() {
            Opcode::CONTINUATION_RESUME => SuspensionEdge::Resume,
            Opcode::CONTINUATION_COMPLETE => SuspensionEdge::Complete,
            _ => unreachable!("continuation transfer selects one control opcode"),
        };
        let point = self
            .continuations
            .get(id)
            .ok_or_else(Error::invalid_instruction)
            .and_then(|continuation| self.machine.continuation_point(continuation))?;
        if matches!(
            (point, edge),
            (FramePoint::Entry { .. }, SuspensionEdge::Complete)
        ) && returned_registers.word_count != value.word_count
        {
            return Err(self.invalid_instruction());
        }
        let continuation = self
            .continuations
            .take(id)
            .ok_or_else(|| self.invalid_instruction())?;
        let return_to = Return::Continuation {
            pc,
            yielded_registers,
            continuation_register,
            returned_registers,
            yielded: frame.branch_offset(yielded),
            returned: frame.branch_offset(returned),
            unwind: Some(frame.branch_offset(unwind)),
        };

        // enter an initial body or complete it without executing its first instruction
        match (point, edge) {
            (FramePoint::Entry { .. }, SuspensionEdge::Resume) => {
                self.save_position();
                self.machine.attach_continuation(continuation, return_to)?;
                self.activate();

                Ok(())
            }
            (FramePoint::Entry { .. }, SuspensionEdge::Complete) => {
                let values = self
                    .machine
                    .stack
                    .words(frame.range(value), value.word_count as usize);
                let target = frame.range(returned_registers);
                for (index, value) in values.into_iter().enumerate() {
                    self.machine.stack.write(target + index, value);
                }
                self.jump(frame.branch_offset(returned));

                self.destroy_continuation(pc, continuation)
            }
            (FramePoint::Operation(_), edge) => {
                let values = self
                    .machine
                    .stack
                    .words(frame.range(value), value.word_count as usize);
                self.save_position();
                self.machine.attach_continuation(continuation, return_to)?;
                self.enter_suspension(&values, edge)
            }
            (FramePoint::Entry { .. }, SuspensionEdge::Cancel) => {
                unreachable!("continuation control cannot cancel")
            }
        }
    }
}
