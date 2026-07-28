use destack_bytecode::{CodeOffset, Instruction, Opcode, RegisterSpan};
use destack_program::{
    Continuation, ContinuationId, FramePoint, FunctionId, Runtime, Task, TypeId, Waiter, Word,
};

use crate::diagnostic::{Error, ExecutionError, ExecutionResult, Result};
use crate::machine::{Activation, Return};

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one task operation.
    pub(crate) fn execute_task(
        &mut self,
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        match instruction.opcode() {
            Opcode::TASK_RESOLVE => self.execute_task_resolve(instruction),
            Opcode::TASK_START => self.execute_task_start(pc, instruction),
            Opcode::TASK_PARK => self.execute_task_park(instruction),
            Opcode::TASK_CANCEL => self.execute_task_cancel(instruction),
            Opcode::TASK_DETACH => self.execute_task_detach(instruction),
            _ => unreachable!("task dispatch selects one task opcode"),
        }
    }

    /// Create one already completed task.
    fn execute_task_resolve(
        &mut self,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        let mut operands = self.operands(instruction);
        let destination = operands.register()?;
        let ty = TypeId(operands.u32()?);
        let value = operands.span()?;
        let frame = self.frame();
        let words = self
            .machine
            .stack
            .words(frame.range(value), value.word_count as usize);
        let value = self
            .machine
            .program
            .value(ty, words)
            .map_err(Error::program)?;
        let task = self.activation.runtime.resolve_task(value);

        self.write(destination.0, Word::from_bits(task.bits()));

        Ok(())
    }

    /// Start one ready continuation as an eager task.
    fn execute_task_start(
        &mut self,
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        let mut operands = self.operands(instruction);
        let destination = operands.register()?;
        let continuation = operands.register()?;
        let continuation = ContinuationId::from_word(self.read(continuation.0));
        let continuation = self
            .continuations
            .take(continuation)
            .ok_or_else(|| self.invalid_instruction())?;
        if !matches!(
            self.machine.continuation_point(&continuation)?,
            FramePoint::Entry { .. }
        ) {
            return Err(self.invalid_instruction().into());
        }
        let return_to = Return::Task {
            pc,
            task_register: destination.0,
        };
        let caller = self.frame();

        // restore physical execution before publishing one live runtime task
        self.save_position();
        self.machine.attach_continuation(&continuation, return_to)?;
        let task = self.activation.runtime.start_task();

        // publish the handle before the eager body can stop for inspection
        self.machine
            .stack
            .write(caller.register(destination.0), Word::from_bits(task.bits()));
        self.activate();

        Ok(())
    }

    /// Park one waiter until a task completes or is cancelled.
    fn execute_task_park(&mut self, instruction: Instruction<'_>) -> ExecutionResult<(), R::Error> {
        let (task, waiter) = self.task_and_waiter(instruction)?;

        self.activation
            .runtime
            .park_task(task, waiter)
            .map_err(ExecutionError::runtime)
    }

    /// Request cooperative cancellation of one task.
    fn execute_task_cancel(
        &mut self,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        let task = self.task(instruction)?;

        self.activation
            .runtime
            .cancel_task(task)
            .map_err(ExecutionError::runtime)
    }

    /// Detach one task result.
    fn execute_task_detach(
        &mut self,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        let task = self.task(instruction)?;

        self.activation
            .runtime
            .detach_task(task)
            .map_err(ExecutionError::runtime)
    }

    /// Decode one task handle operand.
    fn task(&self, instruction: Instruction<'_>) -> Result<Task> {
        let mut operands = self.operands(instruction);
        let task = operands.register()?;

        Ok(Task::from_bits(self.read(task.0).bits()))
    }

    /// Decode one task and waiter operand pair.
    fn task_and_waiter(&self, instruction: Instruction<'_>) -> Result<(Task, Waiter)> {
        let mut operands = self.operands(instruction);
        let task = operands.register()?;
        let waiter = operands.register()?;
        let task = Task::from_bits(self.read(task.0).bits());
        let waiter = Waiter::from_bits(self.read(waiter.0).bits());

        Ok((task, waiter))
    }

    /// Enter one concrete Awaitable park implementation above the task caller.
    pub(super) fn park_task_awaitable(
        &mut self,
        pc: CodeOffset,
        park: FunctionId,
        mut arguments: Vec<Word>,
        task: Task,
        continuation: Continuation,
        first_frame: usize,
    ) -> ExecutionResult<(), R::Error> {
        let return_to = Return::Call {
            pc,
            registers: RegisterSpan::empty(),
            normal: None,
            unwind: None,
        };
        let stack_byte_len = self.machine.stack.byte_len();
        let frame_byte_offset = self.machine.frames[first_frame].byte_offset();
        self.machine.stack.truncate(frame_byte_offset);
        let frame =
            match self
                .machine
                .reserve_frame(first_frame, park, arguments.len() + 1, return_to)
            {
                Ok(frame) => frame,
                Err(error) => {
                    self.machine.stack.grow(stack_byte_len)?;

                    return Err(error.into());
                }
            };

        // publish suspension only after the park frame can be entered
        let waiter = match self.activation.runtime.suspend_task(task, continuation) {
            Ok(waiter) => waiter,
            Err(error) => {
                self.machine.stack.grow(stack_byte_len)?;

                return Err(ExecutionError::runtime(error));
            }
        };
        arguments.push(Word::from_bits(waiter.bits()));

        // commit the captured suffix and initialize its replacement frame
        self.machine.frames.truncate(first_frame);
        for (index, argument) in arguments.iter().copied().enumerate() {
            self.machine
                .stack
                .write(frame.register_offset + index, argument);
        }
        self.machine.frames.push(frame);
        self.activate();

        Ok(())
    }
}
