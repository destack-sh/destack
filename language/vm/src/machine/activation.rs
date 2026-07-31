use std::ops::Range;
use std::ptr;

use destack_bytecode::{CodeOffset, Instruction, Operands, RegisterId, RegisterSpan};
use destack_heap::{HeapResult, RootSlot};
use destack_program as program;
use destack_program::{
    Completion, ContinuationTable, FrameStateId, FunctionId, Outcome, Profile, ProgramPoint,
    ResumeSkip, Runtime, StopSet, WatchSet, Word,
};

use crate::diagnostic::{
    DiagnosticAnchor, Error, ErrorReason, ExecutionError, ExecutionResult, Result, StackTraceFrame,
};

use super::{Callee, Cursor, Frame, Machine, Return, Stack};

/// One active execution over mutable machine state.
pub(crate) struct Activation<'machine, 'run, R>
where
    R: Runtime + ?Sized,
{
    /// The machine being executed.
    pub(crate) machine: &'machine mut Machine,
    /// Worker-local continuation storage.
    pub(crate) continuations: &'machine mut ContinuationTable,
    /// The runtime activation available to this execution.
    pub(crate) activation: program::Activation<'run, 'run, R>,
    /// Native addresses derived for the active frame.
    pub(crate) cursor: Cursor,
    /// The number of instructions executed by this activation.
    pub(crate) instruction_count: u64,
    /// The active instruction byte offset before execution advances.
    pub(crate) pc: CodeOffset,
    /// Active instruction stop points.
    pub(crate) stop_points: Option<&'run StopSet>,
    /// Active memory watchpoints.
    pub(crate) watch_points: Option<&'run WatchSet>,
    /// Profile receiving execution measurements.
    pub(crate) profile: Option<&'run mut Profile>,
    /// One retained stop skipped on continued execution.
    pub(crate) resume_skip: Option<ResumeSkip>,
    /// The located panic currently unwinding this activation.
    pub(crate) panic: Option<Error>,
}

impl<'machine, 'run, R> Activation<'machine, 'run, R>
where
    R: Runtime + ?Sized,
{
    /// Bind one activation to a machine.
    pub(crate) const fn new(
        machine: &'machine mut Machine,
        continuations: &'machine mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
    ) -> Self {
        Self {
            machine,
            continuations,
            activation,
            cursor: Cursor::dangling(),
            instruction_count: 0,
            pc: CodeOffset(0),
            stop_points: None,
            watch_points: None,
            profile: None,
            resume_skip: None,
            panic: None,
        }
    }

    /// Attach debugger and profiler instrumentation.
    pub(crate) fn instrument(
        mut self,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
        resume_skip: Option<ResumeSkip>,
    ) -> Self {
        self.stop_points = stop_points;
        self.watch_points = watch_points;
        self.profile = profile;
        self.resume_skip = resume_skip;

        self
    }

    /// Return one complete register byte range in the active frame.
    pub(crate) fn register_byte_range(&self, registers: RegisterSpan) -> Result<Range<usize>> {
        let frame = self.frame();
        let end = registers.start.0 as usize + registers.word_count as usize;
        if end > frame.register_count as usize {
            return Err(self.invalid_instruction());
        }

        let start = frame.range(registers) * Word::BYTE_LEN;
        let byte_len = registers.word_count as usize * Word::BYTE_LEN;

        Ok(start..start + byte_len)
    }

    /// Execute one entry function to completion.
    pub(crate) fn run(
        mut self,
        function: FunctionId,
        arguments: &[Word],
    ) -> ExecutionResult<Outcome<Vec<Word>>, R::Error>
    where
        R::Error: From<Error>,
    {
        if let Callee::Binding(binding) =
            Callee::resolve(&self.machine.program, self.machine.bytecode, function)?
        {
            let result_count = self.binding_result_word_count(function)?;
            let mut result = vec![Word::ZERO; result_count];
            let memory = self.activation.memory.reborrow();
            self.activation
                .runtime
                .call_binding(memory, binding, arguments, &mut result)
                .map_err(ExecutionError::runtime)?;

            return Ok(Outcome::Completed { value: result });
        }
        let return_to = Return::Exit {
            completion: Completion::Return,
        };
        self.push_frame(function, arguments, return_to)?;

        self.execute()
    }

    /// Dispatch through the exact configured execution loop.
    pub(crate) fn execute(&mut self) -> ExecutionResult<Outcome<Vec<Word>>, R::Error>
    where
        R::Error: From<Error>,
    {
        self.activate();

        let is_stopping = self.stop_points.is_some_and(|points| !points.is_empty());
        let is_watching = self.watch_points.is_some_and(|points| !points.is_empty());
        let is_profiling = self.profile.is_some();
        let is_bounded = self.machine.limits.max_instructions.is_some();

        let result = if is_bounded {
            self.select_dispatch::<true>(is_stopping, is_watching, is_profiling)
        } else {
            self.select_dispatch::<false>(is_stopping, is_watching, is_profiling)
        };

        result.map_err(|error| match error {
            ExecutionError::Machine(error) if matches!(error.reason(), ErrorReason::Panic(_)) => {
                ExecutionError::Machine(error)
            }
            ExecutionError::Machine(error) => ExecutionError::Machine(self.locate(error)),
            ExecutionError::Runtime(error) => ExecutionError::Runtime(error),
        })
    }

    /// Select and enter the exact active dispatch loop.
    fn select_dispatch<const BOUNDED: bool>(
        &mut self,
        is_stopping: bool,
        is_watching: bool,
        is_profiling: bool,
    ) -> ExecutionResult<Outcome<Vec<Word>>, R::Error>
    where
        R::Error: From<Error>,
    {
        match (is_stopping, is_watching, is_profiling) {
            (false, false, false) => self.dispatch::<false, false, false, BOUNDED>(),
            (false, false, true) => self.dispatch::<false, false, true, BOUNDED>(),
            (false, true, false) => self.dispatch::<false, true, false, BOUNDED>(),
            (false, true, true) => self.dispatch::<false, true, true, BOUNDED>(),
            (true, false, false) => self.dispatch::<true, false, false, BOUNDED>(),
            (true, false, true) => self.dispatch::<true, false, true, BOUNDED>(),
            (true, true, false) => self.dispatch::<true, true, false, BOUNDED>(),
            (true, true, true) => self.dispatch::<true, true, true, BOUNDED>(),
        }
    }

    /// Service one pending runtime poll against the live VM activation.
    #[cold]
    pub(crate) fn poll(&mut self, state: FrameStateId) -> ExecutionResult<program::Poll, R::Error>
    where
        R::Error: From<Error>,
    {
        let machine = &mut *self.machine;
        let continuations = &mut *self.continuations;
        let program = machine.program.clone();
        let mut roots = |visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>| {
            machine
                .visit_root_slots_at(state, visit)
                .map_err(R::Error::from)?;
            continuations
                .visit_root_slots(&program, &machine.stack.memory(), visit)
                .map_err(Error::program)
                .map_err(R::Error::from)?;
            Ok(())
        };
        let memory = self.activation.memory.reborrow();

        self.activation
            .runtime
            .poll(memory, &mut roots)
            .map_err(ExecutionError::runtime)
    }

    /// Push one linked bytecode frame.
    pub(crate) fn push_frame(
        &mut self,
        function: FunctionId,
        arguments: &[Word],
        return_to: Return,
    ) -> ExecutionResult<(), R::Error> {
        let frame = self
            .machine
            .allocate_frame(function, arguments.len(), return_to)?;

        // initialize argument registers in calling order
        for (index, argument) in arguments.iter().copied().enumerate() {
            self.machine
                .stack
                .write(frame.register_offset + index, argument);
        }
        self.machine.frames.push(frame);
        self.activate();

        Ok(())
    }

    /// Enter one bytecode call from the active frame.
    pub(crate) fn call(
        &mut self,
        function: FunctionId,
        arguments: RegisterSpan,
        environment: Option<Word>,
        return_to: Return,
        normal_displacement: Option<i32>,
        unwind_displacement: Option<i32>,
    ) -> ExecutionResult<(), R::Error> {
        if self
            .machine
            .program
            .function(function)
            .is_some_and(|function| function.coroutine().is_some())
        {
            return Err(self.invalid_instruction().into());
        }
        let caller = self.frame();
        let argument_start = caller.range(arguments);
        let environment_word_count = usize::from(environment.is_some());
        let initialized_word_count = arguments.word_count as usize + environment_word_count;
        let normal = normal_displacement.map(|offset| Self::branch_offset(caller, offset));
        let unwind = unwind_displacement.map(|offset| Self::branch_offset(caller, offset));
        let return_to = match return_to {
            Return::Call { pc, registers, .. } => Return::Call {
                pc,
                registers,
                normal,
                unwind,
            },
            Return::Exit { .. } | Return::Continuation { .. } | Return::Task { .. } => {
                return Err(self.invalid_instruction().into());
            }
            Return::Drop {
                pc,
                caller_state,
                frame_count,
            } => Return::Drop {
                pc,
                caller_state,
                frame_count,
            },
        };
        if let Callee::Binding(binding) =
            Callee::resolve(&self.machine.program, self.machine.bytecode, function)?
        {
            let Return::Call {
                registers, normal, ..
            } = return_to
            else {
                unreachable!("ordinary binding calls require one call return");
            };
            let result_count = registers.word_count as usize;
            let frame = self.frame();
            Self::call_binding(
                frame,
                &self.machine.stack,
                &mut self.machine.binding_buffer,
                &mut self.activation,
                binding,
                environment,
                arguments,
                result_count,
            )?;

            // publish binding results directly into the caller frame
            for index in 0..result_count {
                let value = self.machine.binding_buffer[index];
                self.write(registers.start.0 + index as u16, value);
            }
            self.machine.binding_buffer.clear();
            if let Some(normal) = normal {
                self.jump(normal);
            }

            return Ok(());
        }
        let frame = self
            .machine
            .allocate_frame(function, initialized_word_count, return_to)?;

        // transfer arguments directly between contiguous stack windows
        if let Some(environment) = environment {
            self.machine.stack.write(frame.register_offset, environment);
        }
        let argument_offset = frame.register_offset + environment_word_count;
        self.machine.stack.copy_words(
            argument_start,
            argument_offset,
            arguments.word_count as usize,
        );
        self.save_position();
        self.machine.frames.push(frame);
        self.activate();

        Ok(())
    }

    /// Call one destructor with an exclusive reference to caller storage.
    pub(crate) fn call_destructor(
        &mut self,
        function: FunctionId,
        value_offset: usize,
        pc: CodeOffset,
        caller_state: FrameStateId,
        frame_count: u16,
    ) -> Result<()> {
        let Some(_caller) = self.machine.frames.last().copied() else {
            unreachable!("destructors require an active caller");
        };
        let return_to = Return::Drop {
            pc,
            caller_state,
            frame_count,
        };
        let frame = self.machine.allocate_frame(function, 1, return_to)?;
        let reference = value_offset + 2;

        // pass one non-null stack-relative reference to retained value storage
        self.machine
            .stack
            .write(frame.register_offset, Word::from_bits(reference as u64));
        self.save_position();
        self.machine.frames.push(frame);
        self.activate();

        Ok(())
    }

    /// Replace the active frame with one tail-called function.
    pub(crate) fn tail_call(
        &mut self,
        function: FunctionId,
        arguments: RegisterSpan,
        environment: Option<Word>,
    ) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
        if self
            .machine
            .program
            .function(function)
            .is_some_and(|function| function.coroutine().is_some())
        {
            return Err(self.invalid_instruction().into());
        }
        let current = self.frame();
        let argument_start = current.range(arguments);
        let callee = Callee::resolve(&self.machine.program, self.machine.bytecode, function)?;
        if let Callee::Binding(binding) = callee {
            let result_count = self.binding_result_word_count(function)?;
            Self::call_binding(
                current,
                &self.machine.stack,
                &mut self.machine.binding_buffer,
                &mut self.activation,
                binding,
                environment,
                arguments,
                result_count,
            )?;
            let Ok(result_word_count) = u16::try_from(result_count) else {
                return Err(self.invalid_instruction().into());
            };
            let byte_len = result_count * Word::BYTE_LEN;
            self.machine.stack.grow(current.byte_offset() + byte_len)?;
            for index in 0..result_count {
                let value = self.machine.binding_buffer[index];
                self.machine
                    .stack
                    .write(current.register_offset + index, value);
            }
            self.machine.binding_buffer.clear();
            let results = RegisterSpan::new(RegisterId(0), result_word_count);

            return self.return_frame(results);
        }
        let Callee::Bytecode {
            function: linked,
            code,
        } = callee
        else {
            unreachable!("binding implementation returned above");
        };
        let register_count = linked.register_count;
        let environment_word_count = usize::from(environment.is_some());
        let initialized_word_count = arguments.word_count as usize + environment_word_count;
        if initialized_word_count > register_count as usize {
            return Err(self.invalid_instruction().into());
        }

        // resize the current register window for the replacement frame
        let register_offset = current.register_offset;
        let register_byte_offset = register_offset * Word::BYTE_LEN;
        let end = register_byte_offset + register_count as usize * Word::BYTE_LEN;
        self.machine.stack.grow(end)?;

        // move arguments before replacing bytes that may overlap their source
        let argument_offset = register_offset + environment_word_count;
        self.machine.stack.move_words(
            argument_start,
            argument_offset,
            arguments.word_count as usize,
        );

        // initialize the hidden environment after moving overlapping arguments
        if let Some(environment) = environment {
            self.machine.stack.write(register_offset, environment);
        }
        self.machine.stack.truncate(end);

        // retain only the original caller transition
        let frame = Frame::new(
            function,
            code,
            register_offset,
            register_count,
            current.return_to,
        );
        self.cursor.replace_frame(frame);
        self.activate();

        Ok(None)
    }

    /// Call one runtime binding with flattened frame arguments.
    fn call_binding(
        frame: Frame,
        stack: &Stack,
        words: &mut Vec<Word>,
        activation: &mut program::Activation<'run, 'run, R>,
        binding: &program::Binding,
        environment: Option<Word>,
        arguments: RegisterSpan,
        result_count: usize,
    ) -> ExecutionResult<(), R::Error> {
        let argument_start = frame.range(arguments);
        let environment_count = usize::from(environment.is_some());
        let argument_count = environment_count + arguments.word_count as usize;
        words.clear();
        words.reserve(argument_count + result_count);

        // flatten the hidden environment before source arguments
        words.extend(environment);
        words.extend(stack.words(argument_start, arguments.word_count as usize));
        words.resize(argument_count + result_count, Word::ZERO);

        // invoke through the exact runtime error boundary
        let (arguments, result) = words.split_at_mut(argument_count);
        let memory = activation.memory.reborrow();
        activation
            .runtime
            .call_binding(memory, binding, arguments, result)
            .map_err(ExecutionError::runtime)?;

        // retain only returned words in the existing allocation
        words.copy_within(argument_count.., 0);
        words.truncate(result_count);

        Ok(())
    }

    /// Return the exact result word count for one bound function.
    fn binding_result_word_count(&self, function: FunctionId) -> Result<usize> {
        self.machine
            .program
            .function_result_word_count(function)
            .ok_or_else(|| self.invalid_instruction())
    }

    /// Return one register range from the current frame.
    pub(crate) fn return_frame(
        &mut self,
        results: RegisterSpan,
    ) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
        let Some(frame) = self.machine.frames.pop() else {
            unreachable!("bytecode returns require an active frame");
        };
        let result_start = frame.range(results);
        if !self.machine.frames.is_empty() {
            self.activate();
        }

        match frame.return_to {
            // publish the root frame outcome
            Return::Exit { completion } => {
                let values = self
                    .machine
                    .stack
                    .words(result_start, results.word_count as usize);
                self.machine.stack.truncate(frame.byte_offset());
                let outcome = match completion {
                    Completion::Return => Outcome::Completed { value: values },
                    Completion::Cancel => Outcome::Cancelled,
                };

                Ok(Some(outcome))
            }

            // copy ordinary call results into the caller register window
            Return::Call {
                registers, normal, ..
            } => {
                if results.word_count != registers.word_count {
                    return Err(self.invalid_instruction().into());
                }
                let Some(caller) = self.machine.frames.last().copied() else {
                    unreachable!("called frames require a caller");
                };
                let target = caller.range(registers);
                self.machine
                    .stack
                    .copy_words(result_start, target, results.word_count as usize);
                if let Some(normal) = normal {
                    self.jump(normal);
                }
                self.machine.stack.truncate(frame.byte_offset());

                Ok(None)
            }

            // enter the final-return successor of a continuation drive
            Return::Continuation {
                returned_registers,
                returned,
                ..
            } => {
                if results.word_count != returned_registers.word_count {
                    return Err(self.invalid_instruction().into());
                }
                let Some(caller) = self.machine.frames.last().copied() else {
                    unreachable!("continued frames require a caller");
                };
                let target = caller.range(returned_registers);
                self.machine
                    .stack
                    .copy_words(result_start, target, results.word_count as usize);
                self.jump(returned);
                self.machine.stack.truncate(frame.byte_offset());

                Ok(None)
            }

            // settle the eagerly started task owned by the caller
            Return::Task { task_register, .. } => {
                let Some(caller) = self.machine.frames.last().copied() else {
                    unreachable!("task frames require a caller");
                };
                let task = program::Task::from_bits(
                    self.machine
                        .stack
                        .read(caller.register(task_register))
                        .bits(),
                );
                let result_type = self
                    .machine
                    .program
                    .function_result(frame.function)
                    .ok_or_else(|| self.invalid_instruction())?;
                let words = self
                    .machine
                    .stack
                    .words(result_start, results.word_count as usize);
                let value = self
                    .machine
                    .program
                    .value(result_type, words)
                    .map_err(Error::program)?;
                let is_cancelled = self
                    .activation
                    .runtime
                    .is_task_cancelled(task)
                    .map_err(ExecutionError::runtime)?;

                // publish the exact terminal task state
                let outcome = if is_cancelled {
                    program::TaskOutcome::Cancelled
                } else {
                    program::TaskOutcome::Completed(value)
                };
                self.activation
                    .runtime
                    .finish_task(task, outcome)
                    .map_err(ExecutionError::runtime)?;
                self.machine.stack.truncate(frame.byte_offset());

                Ok(None)
            }

            // release one destructor frame or its retained continuation frames
            Return::Drop { frame_count, .. } => {
                if results.word_count != 0 {
                    return Err(self.invalid_instruction().into());
                }
                let stack_byte_len = if frame_count == 0 {
                    frame.byte_offset()
                } else {
                    let first_frame = self
                        .machine
                        .frames
                        .len()
                        .checked_sub(frame_count as usize)
                        .ok_or_else(|| self.invalid_instruction())?;
                    let Some(first) = self.machine.frames.get(first_frame).copied() else {
                        return Err(self.invalid_instruction().into());
                    };
                    self.machine.frames.truncate(first_frame);

                    first.byte_offset()
                };
                self.machine.stack.truncate(stack_byte_len);
                if !self.machine.frames.is_empty() {
                    self.activate();
                }

                Ok(None)
            }
        }
    }

    /// Return the active frame by value.
    #[inline(always)]
    pub(crate) fn frame(&self) -> Frame {
        self.cursor.frame()
    }

    /// Save the live instruction offset in the active canonical frame.
    #[inline(always)]
    pub(crate) fn save_position(&mut self) {
        self.cursor.save_position();
    }

    /// Capture execution at one canonical frame state and release physical frames.
    pub(crate) fn capture(&mut self, pc: CodeOffset) -> Result<()> {
        let frame = self.frame();
        let state = self.machine.frame_state_at(frame, pc)?;
        self.save_position();

        self.machine.capture(state)
    }

    /// Derive native execution addresses for the active canonical frame.
    pub(crate) fn activate(&mut self) {
        let Some(frame) = self.machine.frames.last_mut() else {
            unreachable!("bytecode execution requires an active frame");
        };
        let active = *frame;
        let frame = ptr::from_mut(frame);
        let sections = self.machine.program.sections();
        let bytes = self.machine.bytecode.bytes(sections);

        // materialize native addresses only for this activation
        let code = unsafe { bytes.as_ptr().add(active.code.byte_offset as usize) };
        let registers = self.machine.stack.address(active.byte_offset()) as *mut Word;

        // SAFETY: linked frame ranges address Program code and live stack registers
        unsafe {
            self.cursor.set(code, active.pc, registers, frame);
        }
    }

    /// Advance the active canonical frame and native cursor.
    #[inline(always)]
    pub(crate) fn advance(&mut self, byte_len: usize) {
        self.cursor.advance(byte_len);
    }

    /// Branch from the current instruction successor.
    #[inline(always)]
    pub(crate) fn branch(&mut self, displacement: i32) {
        self.cursor.branch(displacement);
    }

    /// Enter one exact function-relative byte offset.
    #[inline(always)]
    pub(crate) fn jump(&mut self, pc: CodeOffset) {
        self.cursor.jump(pc);
    }

    /// Read one active frame register.
    #[inline(always)]
    pub(crate) fn read(&self, register: u16) -> Word {
        self.cursor.read(register)
    }

    /// Write one active frame register.
    #[inline(always)]
    pub(crate) fn write(&mut self, register: u16, value: Word) {
        self.cursor.write(register, value);
    }

    /// Read operands from one linked instruction.
    #[inline(always)]
    pub(crate) fn operands<'code>(
        &self,
        instruction: Instruction<'code>,
    ) -> Operands<'code, false> {
        // SAFETY: Program linking establishes each opcode's exact operand layout
        unsafe { instruction.operands_unchecked() }
    }

    /// Move one possibly overlapping active register range.
    pub(crate) fn move_range(&mut self, source: RegisterSpan, target: RegisterSpan) {
        if source.word_count != target.word_count {
            unreachable!("linked range moves preserve register width");
        }

        self.cursor
            .move_words(source.start.0, target.start.0, source.word_count);
    }

    /// Return one invalid instruction error at the active bytecode point.
    #[inline(never)]
    pub(crate) fn invalid_instruction(&self) -> Error {
        Error::invalid_instruction()
    }

    /// Return the Program point at one exact bytecode operation offset when present.
    pub(crate) fn point_at(&self, frame: Frame, offset: CodeOffset) -> Option<ProgramPoint> {
        let program = &self.machine.program;
        let operation = self.machine.bytecode.operation_at(
            program.sections(),
            frame.function.index(),
            offset,
        )?;

        Some(ProgramPoint::new(frame.function, operation))
    }

    /// Return the required Program point at one exact bytecode operation offset.
    pub(crate) fn point(&self, frame: Frame, offset: CodeOffset) -> Result<ProgramPoint> {
        let Some(point) = self.point_at(frame, offset) else {
            return Err(Error::invalid_instruction());
        };

        Ok(point)
    }

    /// Attach the active program location and call stack to one failure.
    pub(crate) fn locate(&self, error: Error) -> Error {
        let program = &self.machine.program;
        let Some(last) = self.machine.frames.len().checked_sub(1) else {
            unreachable!("located execution failures require an active frame");
        };

        // capture each active bytecode location from entry to failure
        let stack = self
            .machine
            .frames
            .iter()
            .copied()
            .enumerate()
            .map(|(index, frame)| {
                let pc = if index == last { self.pc } else { frame.pc };
                let function_name = program
                    .function(frame.function)
                    .and_then(|function| program.string(function.name))
                    .map(str::to_owned);

                StackTraceFrame {
                    function: frame.function,
                    code_offset: pc,
                    function_name,
                }
            })
            .collect();

        // attach the engine-neutral point when the operation has one
        let anchor = self
            .machine
            .frames
            .last()
            .copied()
            .and_then(|frame| self.point_at(frame, self.pc))
            .map_or(DiagnosticAnchor::None, DiagnosticAnchor::Point);

        error.with_stack(stack).with_anchor(anchor)
    }

    /// Return one end-relative branch target in the caller function.
    fn branch_offset(frame: Frame, displacement: i32) -> CodeOffset {
        frame.branch_offset(displacement)
    }
}

impl<R: Runtime + ?Sized> Drop for Activation<'_, '_, R> {
    /// Release physical state after completed or failed execution.
    fn drop(&mut self) {
        self.machine.clear_physical();
    }
}
