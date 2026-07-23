use std::ops::Range;

use destack_bytecode::{Body, CodeOffset, CodeRange, RegisterRange};
use destack_program::{
    Continuation, FrameStateId, FunctionId, Outcome, Profile, ProgramActivation, ProgramPoint,
    ResumeSkip, StopSet, WatchSet, Word,
};

use crate::diagnostic::{DiagnosticAnchor, Error, ErrorReason, Result, StackTraceFrame};

use super::{Frame, Machine, Return};

/// One active execution over mutable machine state.
pub(crate) struct Activation<'machine, 'run> {
    /// The machine being executed.
    pub(crate) machine: &'machine mut Machine,
    /// The runtime call available to this activation.
    pub(crate) call: ProgramActivation<'run>,
    /// The number of instructions executed by this activation.
    pub(crate) instruction_count: u64,
    /// The active instruction byte offset before execution advances.
    pub(crate) instruction_offset: CodeOffset,
    /// Active instruction stop points.
    pub(crate) stop_points: Option<&'run StopSet>,
    /// Active memory watchpoints.
    pub(crate) watch_points: Option<&'run WatchSet>,
    /// Profile receiving execution measurements.
    pub(crate) profile: Option<&'run mut Profile>,
    /// One retained stop skipped on continuation entry.
    pub(crate) resume_skip: Option<ResumeSkip>,
    /// The located panic currently unwinding this activation.
    pub(crate) panic: Option<Error>,
}

impl<'machine, 'run> Activation<'machine, 'run> {
    /// Bind one activation to a machine.
    pub(crate) const fn new(
        machine: &'machine mut Machine,
        call: ProgramActivation<'run>,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
        resume_skip: Option<ResumeSkip>,
    ) -> Self {
        Self {
            machine,
            call,
            instruction_count: 0,
            instruction_offset: CodeOffset(0),
            stop_points,
            watch_points,
            profile,
            resume_skip,
            panic: None,
        }
    }

    /// Return one linked bytecode body.
    pub(crate) fn body(&self, function: FunctionId) -> Result<&Body> {
        let program = &self.machine.program;

        program
            .bytecode()
            .body(program.sections(), function.index())
            .ok_or_else(|| Error::undefined_function(function))
    }

    /// Return executable code for one bytecode function.
    fn code(&self, function: FunctionId) -> Result<(&Body, CodeRange)> {
        let body = self.body(function)?;
        if let Some(code) = body.code() {
            return Ok((body, code));
        }

        let program = &self.machine.program;
        if let Some(binding) = program.function_binding(function) {
            Err(Error::binding_unavailable(function, binding))
        } else {
            Err(Error::undefined_function(function))
        }
    }

    /// Return one complete register byte range in the active frame.
    pub(crate) fn register_byte_range(&self, registers: RegisterRange) -> Result<Range<usize>> {
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
    ) -> Result<Outcome<Continuation, Vec<Word>>> {
        self.push_frame(function, arguments, None)?;

        self.execute()
    }

    /// Dispatch through the exact configured execution loop.
    pub(crate) fn execute(&mut self) -> Result<Outcome<Continuation, Vec<Word>>> {
        let is_stopping = self.stop_points.is_some_and(|points| !points.is_empty());
        let is_watching = self.watch_points.is_some_and(|points| !points.is_empty());
        let is_profiling = self.profile.is_some();
        let is_bounded = self.machine.limits.max_instructions.is_some();

        let result = if is_bounded {
            self.dispatch_configured::<true>(is_stopping, is_watching, is_profiling)
        } else {
            self.dispatch_configured::<false>(is_stopping, is_watching, is_profiling)
        };

        result.map_err(|error| {
            if matches!(&error.reason, ErrorReason::Panic(_)) {
                error
            } else {
                self.locate(error)
            }
        })
    }

    /// Dispatch through the exact active instrumentation set.
    fn dispatch_configured<const BOUNDED: bool>(
        &mut self,
        is_stopping: bool,
        is_watching: bool,
        is_profiling: bool,
    ) -> Result<Outcome<Continuation, Vec<Word>>> {
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

    /// Push one linked bytecode frame.
    pub(crate) fn push_frame(
        &mut self,
        function: FunctionId,
        arguments: &[Word],
        return_to: Option<Return>,
    ) -> Result<()> {
        let frame = self.allocate_frame(function, arguments.len(), return_to)?;

        // initialize argument registers in calling order
        for (index, argument) in arguments.iter().copied().enumerate() {
            self.machine
                .stack
                .write(frame.register_offset + index, argument);
        }
        self.machine.frames.push(frame);

        Ok(())
    }

    /// Enter one bytecode call from the active frame.
    pub(crate) fn call(
        &mut self,
        function: FunctionId,
        arguments: RegisterRange,
        environment: Option<Word>,
        return_to: Return,
        instruction_offset: CodeOffset,
        normal_displacement: Option<i32>,
        unwind_displacement: Option<i32>,
    ) -> Result<()> {
        let Some(caller) = self.machine.frames.last().copied() else {
            unreachable!("bytecode calls require an active caller");
        };
        let argument_start = caller.range(arguments);
        let environment_word_count = usize::from(environment.is_some());
        let initialized_word_count = arguments.word_count as usize + environment_word_count;
        let frame = self.allocate_frame(function, initialized_word_count, Some(return_to))?;

        // retain the caller state before entering the callee
        self.suspend_caller(
            caller,
            instruction_offset,
            normal_displacement,
            unwind_displacement,
        )?;

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
        self.machine.frames.push(frame);

        Ok(())
    }

    /// Call one destructor with an exclusive reference to caller storage.
    pub(crate) fn call_destructor(
        &mut self,
        function: FunctionId,
        value_offset: usize,
        instruction_offset: CodeOffset,
    ) -> Result<()> {
        let Some(caller) = self.machine.frames.last().copied() else {
            unreachable!("destructors require an active caller");
        };
        let frame = self.allocate_frame(function, 1, Some(Return::Drop))?;

        // retain the caller before exposing its stable register address
        self.suspend_caller(caller, instruction_offset, None, None)?;
        let address = self.machine.stack.address(value_offset);
        self.machine
            .stack
            .write(frame.register_offset, Word::from_bits(address as u64));
        self.machine.frames.push(frame);

        Ok(())
    }

    /// Retain one caller's canonical state while its callee is active.
    fn suspend_caller(
        &mut self,
        caller: Frame,
        instruction_offset: CodeOffset,
        normal_displacement: Option<i32>,
        unwind_displacement: Option<i32>,
    ) -> Result<()> {
        let point = self.point(caller, instruction_offset)?;
        let frame_state = self
            .machine
            .program
            .frame_state_at(point)
            .ok_or_else(Error::invalid_continuation)?;
        let normal_state = self.branch_state(caller, normal_displacement)?;
        let unwind_state = self.branch_state(caller, unwind_displacement)?;
        self.frame_mut()
            .suspend(frame_state, normal_state, unwind_state);

        Ok(())
    }

    /// Replace the active frame with one tail-called function.
    pub(crate) fn tail_call(
        &mut self,
        function: FunctionId,
        arguments: RegisterRange,
        environment: Option<Word>,
    ) -> Result<()> {
        let current = self.frame();
        let argument_start = current.range(arguments);
        let (body, code) = self.code(function)?;
        let register_count = body.register_count;
        let environment_word_count = usize::from(environment.is_some());
        let initialized_word_count = arguments.word_count as usize + environment_word_count;
        if initialized_word_count > register_count as usize {
            return Err(self.invalid_instruction());
        }

        // derive the replacement frame ranges from the current stack base
        let frame_layout = self.machine.program.frame_layout(function);
        let frame_byte_len = frame_layout.map_or(0, |layout| layout.byte_len());
        let frame_alignment = frame_layout.map_or(1, |layout| layout.alignment()) as usize;
        let frame_offset = current.stack_offset.next_multiple_of(frame_alignment);
        let register_byte_offset =
            (frame_offset + frame_byte_len as usize).next_multiple_of(Word::BYTE_LEN);
        let register_offset = register_byte_offset / Word::BYTE_LEN;
        let end = register_byte_offset + register_count as usize * Word::BYTE_LEN;
        self.machine.stack.grow(end)?;

        // move arguments before replacing bytes that may overlap their source
        let argument_offset = register_offset + environment_word_count;
        self.machine.stack.move_words(
            argument_start,
            argument_offset,
            arguments.word_count as usize,
        );

        // initialize canonical frame storage without touching dead registers
        self.machine
            .stack
            .zero(frame_offset, frame_byte_len as usize)?;
        if let Some(environment) = environment {
            self.machine.stack.write(register_offset, environment);
        }
        self.machine.stack.truncate(end);

        // retain only the original caller transition
        let frame = Frame::new(
            function,
            code,
            current.stack_offset,
            frame_offset,
            frame_byte_len,
            register_offset,
            register_count,
            current.return_to,
        );
        *self.frame_mut() = frame;

        Ok(())
    }

    /// Return one register range from the current frame.
    pub(crate) fn return_values(&mut self, results: RegisterRange) -> Result<Option<Vec<Word>>> {
        let Some(frame) = self.machine.frames.pop() else {
            unreachable!("bytecode returns require an active frame");
        };
        let result_start = frame.range(results);

        // finish the entry frame at the host boundary
        let Some(return_to) = frame.return_to else {
            let values = self
                .machine
                .stack
                .words(result_start, results.word_count as usize);
            self.machine.stack.truncate(frame.stack_offset);

            return Ok(Some(values));
        };

        // resolve the caller transition before releasing callee storage
        let Some(caller) = self.machine.frames.last().copied() else {
            unreachable!("callee frames require a caller");
        };
        match return_to {
            Return::Values(target) if results.word_count == target.word_count => {
                let target = caller.range(target);
                self.machine
                    .stack
                    .copy_words(result_start, target, results.word_count as usize);
            }
            Return::Drop if results.word_count == 0 => {}
            _ => {
                return Err(Error::invalid_instruction(
                    frame.function,
                    frame.code_offset,
                ));
            }
        }
        self.machine.stack.truncate(frame.stack_offset);

        // enter the normal continuation retained by an invoke
        if let Some(normal_state) = caller.normal_state {
            let point = self
                .machine
                .program
                .frame_point(normal_state)
                .ok_or_else(Error::invalid_continuation)?;
            let code_offset = self.code_offset(point)?;
            self.frame_mut().code_offset = code_offset;
        }
        self.frame_mut().resume();

        Ok(None)
    }

    /// Allocate one fixed register window and frame descriptor.
    pub(crate) fn allocate_frame(
        &mut self,
        function: FunctionId,
        initialized_word_count: usize,
        return_to: Option<Return>,
    ) -> Result<Frame> {
        if self.machine.frames.len() >= self.machine.limits.max_frames {
            return Err(Error::frame_limit_exceeded());
        }

        // resolve the immutable function body and register window
        let (body, code) = self.code(function)?;
        let register_count = body.register_count;
        if initialized_word_count > register_count as usize {
            return Err(Error::invalid_instruction(function, CodeOffset(0)));
        }

        // allocate canonical frame bytes before the register window
        let stack_offset = self.machine.stack.byte_len();
        let frame_layout = self.machine.program.frame_layout(function);
        let frame_byte_len = frame_layout.map_or(0, |layout| layout.byte_len());
        let frame_alignment = frame_layout.map_or(1, |layout| layout.alignment()) as usize;
        let frame_offset = self
            .machine
            .stack
            .push_bytes(frame_byte_len as usize, frame_alignment)?;
        self.machine
            .stack
            .zero(frame_offset, frame_byte_len as usize)?;
        let register_offset = self.machine.stack.push_words(register_count as usize)?;

        Ok(Frame::new(
            function,
            code,
            stack_offset,
            frame_offset,
            frame_byte_len,
            register_offset,
            register_count,
            return_to,
        ))
    }

    /// Return the active frame by value.
    #[inline(always)]
    pub(crate) fn frame(&self) -> Frame {
        let Some(frame) = self.machine.frames.last().copied() else {
            unreachable!("bytecode execution requires an active frame");
        };

        frame
    }

    /// Return the active frame mutably.
    #[inline(always)]
    pub(crate) fn frame_mut(&mut self) -> &mut Frame {
        let Some(frame) = self.machine.frames.last_mut() else {
            unreachable!("bytecode execution requires an active frame");
        };

        frame
    }

    /// Return the active destructor function when one is on the call stack.
    pub(crate) fn destructor(&self) -> Option<FunctionId> {
        self.machine
            .frames
            .iter()
            .rev()
            .copied()
            .find(|frame| frame.is_destructor())
            .map(|frame| frame.function)
    }

    /// Read one active frame register.
    #[inline(always)]
    pub(crate) fn read(&self, register: u16) -> Word {
        let frame = self.frame();

        self.machine.stack.read(frame.register(register))
    }

    /// Write one active frame register.
    #[inline(always)]
    pub(crate) fn write(&mut self, register: u16, value: Word) {
        let frame = self.frame();

        self.machine.stack.write(frame.register(register), value);
    }

    /// Move one possibly overlapping active register range.
    pub(crate) fn move_range(&mut self, source: RegisterRange, target: RegisterRange) {
        if source.word_count != target.word_count {
            unreachable!("linked range moves preserve register width");
        }

        let frame = self.frame();
        self.machine.stack.move_words(
            frame.range(source),
            frame.range(target),
            source.word_count as usize,
        );
    }

    /// Return one invalid instruction error at the active bytecode point.
    pub(crate) fn invalid_instruction(&self) -> Error {
        let frame = self.frame();

        Error::invalid_instruction(frame.function, self.instruction_offset)
    }

    /// Return the Program point at one exact bytecode operation offset when present.
    pub(crate) fn point_at(&self, frame: Frame, offset: CodeOffset) -> Option<ProgramPoint> {
        let program = &self.machine.program;
        let operation =
            program
                .bytecode()
                .operation_at(program.sections(), frame.function.index(), offset)?;

        Some(ProgramPoint::new(frame.function, operation))
    }

    /// Return the required Program point at one exact bytecode operation offset.
    pub(crate) fn point(&self, frame: Frame, offset: CodeOffset) -> Result<ProgramPoint> {
        let Some(point) = self.point_at(frame, offset) else {
            return Err(Error::invalid_instruction(frame.function, offset));
        };

        Ok(point)
    }

    /// Return the bytecode offset for one engine-neutral Program point.
    pub(crate) fn code_offset(&self, point: ProgramPoint) -> Result<CodeOffset> {
        self.machine
            .program
            .bytecode()
            .operation_offset(
                self.machine.program.sections(),
                point.function.index(),
                point.operation,
            )
            .ok_or_else(Error::invalid_continuation)
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
                let code_offset = if index == last {
                    self.instruction_offset
                } else {
                    frame.code_offset
                };
                let function_name = program
                    .function(frame.function)
                    .and_then(|function| program.string(function.name))
                    .map(str::to_owned);

                StackTraceFrame {
                    function: frame.function,
                    code_offset,
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
            .and_then(|frame| self.point_at(frame, self.instruction_offset))
            .map_or(DiagnosticAnchor::None, DiagnosticAnchor::Point);

        error.with_stack(stack).with_anchor(anchor)
    }

    /// Return the frame state selected by one optional call branch.
    fn branch_state(
        &self,
        frame: Frame,
        displacement: Option<i32>,
    ) -> Result<Option<FrameStateId>> {
        let Some(displacement) = displacement else {
            return Ok(None);
        };

        // resolve the branch target through its canonical program point
        let target = CodeOffset(frame.code_offset.0.wrapping_add_signed(displacement));
        let point = self.point(frame, target)?;
        let state = self
            .machine
            .program
            .frame_state_at(point)
            .ok_or_else(Error::invalid_continuation)?;

        Ok(Some(state))
    }
}

impl Drop for Activation<'_, '_> {
    /// Release all ephemeral execution state at the activation boundary.
    fn drop(&mut self) {
        self.machine.frames.clear();
        self.machine.stack.clear();
    }
}
