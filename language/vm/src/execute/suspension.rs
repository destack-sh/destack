use destack_bytecode::{CodeOffset, Instruction, Opcode};
use destack_program::{FrameStateId, FunctionId, Outcome, Suspension, Task, Word};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Return};

/// Successor entered when restoring one suspension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SuspensionEdge {
    /// Enter the normal resumption successor.
    Resume,
    /// Enter the explicit completion successor.
    Complete,
    /// Enter the cancellation cleanup successor.
    Cancel,
}

impl Activation<'_, '_> {
    /// Suspend the active coroutine at one await or yield operation.
    pub(crate) fn execute_suspension(
        &mut self,
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<Option<Outcome<Vec<Word>>>> {
        let mut operands = self.operands(instruction);
        operands.span()?;
        let (operation, park, values) = match instruction.opcode() {
            Opcode::AWAIT => {
                let park = operands.u32()?;
                let awaitable = operands.span()?;

                (Suspension::Await, Some(FunctionId(park)), awaitable)
            }
            Opcode::YIELD => {
                operands.span()?;
                let values = operands.span()?;

                (Suspension::Yield, None, values)
            }
            _ => unreachable!("suspension dispatch selects one suspension opcode"),
        };
        operands.i32()?;
        operands.i32()?;
        operands.i32()?;

        // resolve the canonical site before releasing physical execution
        let frame = self.frame();
        let point = self.point(frame, pc)?;
        let function = self
            .machine
            .program
            .function(frame.function)
            .ok_or_else(|| self.invalid_instruction())?;
        let coroutine = function
            .coroutine()
            .ok_or_else(|| self.invalid_instruction())?;
        let is_supported = match operation {
            Suspension::Await => coroutine.is_async(),
            Suspension::Yield => coroutine.is_generator(),
        };
        if !is_supported {
            return Err(self.invalid_instruction());
        }
        let (_, site) = self
            .machine
            .program
            .suspension(point)
            .ok_or_else(|| self.invalid_instruction())?;
        if site.operation != operation {
            return Err(self.invalid_instruction());
        }
        let state = site.frame_state;
        let value = self
            .machine
            .stack
            .words(frame.range(values), values.word_count as usize);

        // capture from the operation start so resumption can decode the same operation
        self.jump(pc);
        self.save_position();

        match operation {
            Suspension::Await => self.execute_await(pc, state, park, value),
            Suspension::Yield => self.execute_yield(state, value),
        }
    }

    /// Suspend one asynchronous execution or enter requested cancellation.
    fn execute_await(
        &mut self,
        pc: CodeOffset,
        state: FrameStateId,
        park: Option<FunctionId>,
        value: Vec<Word>,
    ) -> Result<Option<Outcome<Vec<Word>>>> {
        let park = park.ok_or_else(|| self.invalid_instruction())?;
        let Some(first_frame) = self
            .machine
            .frames
            .iter()
            .rposition(|frame| matches!(frame.return_to, Return::Task { .. }))
        else {
            let continuation = self.machine.suspend(state)?;

            return Ok(Some(Outcome::Awaited {
                park,
                awaitable: value,
                continuation,
            }));
        };
        let Return::Task {
            pc: task_pc,
            task_register,
        } = self.machine.frames[first_frame].return_to
        else {
            unreachable!("task boundary selection requires one task return");
        };
        let Some(caller) = first_frame
            .checked_sub(1)
            .and_then(|index| self.machine.frames.get(index))
            .copied()
        else {
            return Err(self.invalid_instruction());
        };
        let task = Task::from_bits(
            self.machine
                .stack
                .read(caller.register(task_register))
                .bits(),
        );
        let is_cancelled = self
            .activation
            .runtime
            .is_task_cancelled(task)
            .map_err(Error::program)?;

        // enter cleanup directly when cancellation was requested while running
        if is_cancelled {
            self.jump(pc);
            self.enter_suspension(&[], SuspensionEdge::Cancel)?;

            return Ok(None);
        }

        // capture the task suffix before committing the runtime waiter transition
        let continuation = self.machine.capture_from(first_frame, state)?;
        self.park_task_awaitable(task_pc, park, value, task, continuation, first_frame)?;

        Ok(None)
    }

    /// Return one yield through the nearest continuation control boundary.
    fn execute_yield(
        &mut self,
        state: FrameStateId,
        value: Vec<Word>,
    ) -> Result<Option<Outcome<Vec<Word>>>> {
        let Some(first_frame) = self
            .machine
            .frames
            .iter()
            .rposition(|frame| matches!(frame.return_to, Return::Continuation { .. }))
        else {
            let continuation = self.machine.suspend(state)?;

            return Ok(Some(Outcome::Yielded {
                value,
                continuation,
            }));
        };
        let Return::Continuation {
            yielded_registers,
            continuation_register,
            yielded,
            ..
        } = self.machine.frames[first_frame].return_to
        else {
            unreachable!("continuation boundary selection requires one continuation return");
        };
        if yielded_registers.word_count as usize != value.len() {
            return Err(self.invalid_instruction());
        }
        let continuation = self.machine.suspend_from(first_frame, state)?;
        self.activate();
        let id = self.continuations.insert(continuation);
        let caller = self.frame();
        let target = caller.range(yielded_registers);

        // publish the yielded value and replacement continuation to the caller
        for (index, value) in value.into_iter().enumerate() {
            self.machine.stack.write(target + index, value);
        }
        self.write(continuation_register, id.into_word());
        self.jump(yielded);

        Ok(None)
    }

    /// Resume the restored coroutine through its normal edge.
    pub(crate) fn resume(&mut self, values: &[Word]) -> Result<Outcome<Vec<Word>>> {
        self.enter_suspension(values, SuspensionEdge::Resume)?;

        self.execute()
    }

    /// Complete the restored generator through its explicit completion edge.
    pub(crate) fn complete(&mut self, values: &[Word]) -> Result<Outcome<Vec<Word>>> {
        self.enter_suspension(values, SuspensionEdge::Complete)?;

        self.execute()
    }

    /// Cancel one restored asynchronous suspension through its cleanup edge.
    pub(crate) fn cancel(&mut self) -> Result<Outcome<Vec<Word>>> {
        self.enter_suspension(&[], SuspensionEdge::Cancel)?;

        self.execute()
    }

    /// Enter one successor of the restored suspension.
    pub(super) fn enter_suspension(&mut self, values: &[Word], edge: SuspensionEdge) -> Result<()> {
        self.activate();
        let frame = self.frame();
        let (results, target, byte_len) = {
            let program = &self.machine.program;
            let instruction = program
                .bytecode()
                .instruction(program.sections(), frame.function.index(), frame.pc)?
                .ok_or_else(|| self.invalid_instruction())?;
            let mut operands = self.operands(instruction);
            let resume_results = operands.span()?;
            let (results, target) = match instruction.opcode() {
                Opcode::AWAIT => {
                    operands.u32()?;
                    operands.span()?;
                    let resume = operands.i32()?;
                    let cancel = operands.i32()?;
                    operands.i32()?;

                    let target = match edge {
                        SuspensionEdge::Resume => resume,
                        SuspensionEdge::Cancel => cancel,
                        SuspensionEdge::Complete => return Err(self.invalid_instruction()),
                    };

                    (resume_results, target)
                }
                Opcode::YIELD => {
                    let complete_results = operands.span()?;
                    operands.span()?;
                    let resume = operands.i32()?;
                    let complete = operands.i32()?;
                    operands.i32()?;
                    let (results, target) = match edge {
                        SuspensionEdge::Resume => (resume_results, resume),
                        SuspensionEdge::Complete => (complete_results, complete),
                        SuspensionEdge::Cancel => return Err(self.invalid_instruction()),
                    };

                    (results, target)
                }
                _ => return Err(self.invalid_instruction()),
            };

            (results, target, instruction.byte_len())
        };
        if edge != SuspensionEdge::Cancel && results.word_count as usize != values.len() {
            return Err(self.invalid_instruction());
        }

        // write the delivered value before entering the selected successor
        if edge != SuspensionEdge::Cancel {
            let registers = frame.range(results);
            for (index, value) in values.iter().copied().enumerate() {
                self.machine.stack.write(registers + index, value);
            }
        }
        self.advance(byte_len);
        self.branch(target);
        self.save_position();

        Ok(())
    }
}
