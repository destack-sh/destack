use std::ops::Range;
use std::ptr;

use tspp_bytecode::{CodeOffset, Instruction, Operands, RegisterId, RegisterSpan};
use tspp_program as program;
use tspp_program::{
    Completion, EventSet, FunctionId, Outcome, Profile, ProgramPoint, ResumeSkip, Runtime, StopSet,
    WatchSet, Word,
};

use crate::diagnostic::{
    DiagnosticAnchor, Error, ErrorReason, ExecutionError, ExecutionResult, Result, StackTraceFrame,
};

use super::{Callee, Cursor, Fiber, Frame, Machine, Return};

/// One active execution over one machine and one fiber.
pub(crate) struct Activation<'machine, 'run, R>
where
    R: Runtime + ?Sized,
{
    /// The machine being executed.
    pub(crate) machine: &'machine mut Machine,
    /// The fiber carrying this execution.
    pub(crate) fiber: &'machine mut Fiber,
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
    /// Program event categories selected for observation.
    pub(crate) events: EventSet,
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
    /// Bind one activation to a machine and fiber.
    pub(crate) fn new(
        machine: &'machine mut Machine,
        fiber: &'machine mut Fiber,
        activation: program::Activation<'run, 'run, R>,
    ) -> Self {
        let events = activation.runtime.events();

        Self {
            machine,
            fiber,
            activation,
            cursor: Cursor::dangling(),
            instruction_count: 0,
            pc: CodeOffset(0),
            stop_points: None,
            watch_points: None,
            events,
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
            let binding = *binding;

            // park points require an active frame to receive the wake value
            if binding.is_park() {
                return Err(Error::invalid_instruction().into());
            }
            let result_count = self.binding_result_word_count(function)?;
            let mut result = vec![Word::ZERO; result_count];
            self.observe(program::Event::Binding {
                event: program::BindingEvent::Enter,
                binding_id: binding.id,
            })?;
            let memory = self.activation.memory.reborrow();
            self.activation
                .runtime
                .call_binding(
                    memory,
                    *self.activation.context,
                    self.fiber.fiber_id,
                    &binding,
                    arguments,
                    &mut result,
                )
                .map_err(ExecutionError::runtime)?;
            self.observe(program::Event::Binding {
                event: program::BindingEvent::Exit,
                binding_id: binding.id,
            })?;

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
        let is_observing = !self.events.is_empty();
        let is_profiling = self.profile.is_some();
        let is_bounded = self.machine.limits.max_instructions.is_some();

        let result = if is_bounded {
            self.select_dispatch::<true>(is_stopping, is_watching, is_observing, is_profiling)
        } else {
            self.select_dispatch::<false>(is_stopping, is_watching, is_observing, is_profiling)
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
        is_observing: bool,
        is_profiling: bool,
    ) -> ExecutionResult<Outcome<Vec<Word>>, R::Error>
    where
        R::Error: From<Error>,
    {
        match (is_stopping, is_watching, is_observing, is_profiling) {
            (false, false, false, false) => self.dispatch::<false, false, false, false, BOUNDED>(),
            (false, false, false, true) => self.dispatch::<false, false, false, true, BOUNDED>(),
            (false, false, true, false) => self.dispatch::<false, false, true, false, BOUNDED>(),
            (false, false, true, true) => self.dispatch::<false, false, true, true, BOUNDED>(),
            (false, true, false, false) => self.dispatch::<false, true, false, false, BOUNDED>(),
            (false, true, false, true) => self.dispatch::<false, true, false, true, BOUNDED>(),
            (false, true, true, false) => self.dispatch::<false, true, true, false, BOUNDED>(),
            (false, true, true, true) => self.dispatch::<false, true, true, true, BOUNDED>(),
            (true, false, false, false) => self.dispatch::<true, false, false, false, BOUNDED>(),
            (true, false, false, true) => self.dispatch::<true, false, false, true, BOUNDED>(),
            (true, false, true, false) => self.dispatch::<true, false, true, false, BOUNDED>(),
            (true, false, true, true) => self.dispatch::<true, false, true, true, BOUNDED>(),
            (true, true, false, false) => self.dispatch::<true, true, false, false, BOUNDED>(),
            (true, true, false, true) => self.dispatch::<true, true, false, true, BOUNDED>(),
            (true, true, true, false) => self.dispatch::<true, true, true, false, BOUNDED>(),
            (true, true, true, true) => self.dispatch::<true, true, true, true, BOUNDED>(),
        }
    }

    /// Service one pending runtime poll against the live VM activation.
    #[cold]
    pub(crate) fn poll(&mut self) -> ExecutionResult<program::Poll, R::Error> {
        let memory = self.activation.memory.reborrow();

        self.activation
            .runtime
            .poll(memory)
            .map_err(ExecutionError::runtime)
    }

    /// Push one linked bytecode frame.
    pub(crate) fn push_frame(
        &mut self,
        function: FunctionId,
        arguments: &[Word],
        return_to: Return,
    ) -> ExecutionResult<(), R::Error> {
        let frame =
            self.machine
                .allocate_frame(self.fiber, function, arguments.len(), return_to)?;

        // initialize argument registers in calling order
        for (index, argument) in arguments.iter().copied().enumerate() {
            self.fiber
                .stack
                .write(frame.register_offset + index, argument);
        }
        self.fiber.frames.push(frame);
        self.activate();
        self.observe(program::Event::Frame {
            event: program::FrameEvent::Enter,
            function,
        })?;

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
    ) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
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
            Return::Exit { .. } => {
                return Err(self.invalid_instruction().into());
            }
            Return::Drop { .. } | Return::Release(_) => return_to,
        };
        if let Callee::Binding(binding) =
            Callee::resolve(&self.machine.program, self.machine.bytecode, function)?
        {
            let binding = *binding;

            // boundaries and destructors enter bodies, never bindings
            let Return::Call {
                registers, normal, ..
            } = return_to
            else {
                return Err(self.invalid_instruction().into());
            };

            // park points suspend this fiber instead of entering a host call
            if binding.is_park() {
                return self.park(registers, normal);
            }
            let result_count = registers.word_count as usize;
            let frame = self.frame();
            self.observe(program::Event::Binding {
                event: program::BindingEvent::Enter,
                binding_id: binding.id,
            })?;
            Self::call_binding(
                frame,
                &self.fiber.stack,
                &mut self.machine.binding_buffer,
                &mut self.activation,
                self.fiber.fiber_id,
                &binding,
                environment,
                arguments,
                result_count,
            )?;
            self.observe(program::Event::Binding {
                event: program::BindingEvent::Exit,
                binding_id: binding.id,
            })?;

            // publish binding results directly into the caller frame
            for index in 0..result_count {
                let value = self.machine.binding_buffer[index];
                self.write(registers.start.0 + index as u16, value);
            }
            self.machine.binding_buffer.clear();
            if let Some(normal) = normal {
                self.jump(normal);
            }

            return Ok(None);
        }
        let frame =
            self.machine
                .allocate_frame(self.fiber, function, initialized_word_count, return_to)?;

        // transfer arguments directly between contiguous stack windows
        if let Some(environment) = environment {
            self.fiber.stack.write(frame.register_offset, environment);
        }
        let argument_offset = frame.register_offset + environment_word_count;
        self.fiber.stack.copy_words(
            argument_start,
            argument_offset,
            arguments.word_count as usize,
        );
        self.save_position();
        self.fiber.frames.push(frame);
        self.activate();
        self.observe(program::Event::Frame {
            event: program::FrameEvent::Enter,
            function,
        })?;

        Ok(None)
    }

    /// Ask the runtime to park the running fiber at one binding call.
    fn park(
        &mut self,
        registers: RegisterSpan,
        normal: Option<CodeOffset>,
    ) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
        let fiber_id = self.fiber_id()?;
        let park = self
            .activation
            .runtime
            .park(fiber_id)
            .map_err(ExecutionError::runtime)?;
        match park {
            // deliver an already settled wake like ordinary binding results
            program::Park::Ready(value) => {
                let words = value.words();
                if words.len() != registers.word_count as usize {
                    return Err(self.invalid_instruction().into());
                }
                for (index, word) in words.iter().copied().enumerate() {
                    self.write(registers.start.0 + index as u16, word);
                }
                if let Some(normal) = normal {
                    self.jump(normal);
                }

                Ok(None)
            }
            // retain the post-call continuation and await one wake delivery
            program::Park::Parked => {
                if let Some(normal) = normal {
                    self.jump(normal);
                }
                self.save_position();
                self.fiber.context = *self.activation.context;

                self.fiber.wake_to = Some(registers);

                Ok(Some(Outcome::Parked))
            }
        }
    }

    /// Call one destructor with a mutable reference to caller storage.
    pub(crate) fn call_destructor(
        &mut self,
        function: FunctionId,
        reference: Word,
        return_to: Return,
    ) -> ExecutionResult<(), R::Error> {
        let Some(_caller) = self.fiber.frames.last().copied() else {
            unreachable!("destructors require an active caller");
        };
        let frame = self
            .machine
            .allocate_frame(self.fiber, function, 1, return_to)?;

        // pass the relative reference to retained value storage
        self.fiber.stack.write(frame.register_offset, reference);
        self.save_position();
        self.fiber.frames.push(frame);
        self.activate();
        self.observe(program::Event::Frame {
            event: program::FrameEvent::Enter,
            function,
        })?;

        Ok(())
    }

    /// Replace the active frame with one tail-called function.
    pub(crate) fn tail_call(
        &mut self,
        function: FunctionId,
        arguments: RegisterSpan,
        environment: Option<Word>,
    ) -> ExecutionResult<Option<Outcome<Vec<Word>>>, R::Error> {
        let current = self.frame();
        let argument_start = current.range(arguments);
        let callee = Callee::resolve(&self.machine.program, self.machine.bytecode, function)?;
        if let Callee::Binding(binding) = callee {
            let binding = *binding;

            // park points are never emitted in tail position
            if binding.is_park() {
                return Err(self.invalid_instruction().into());
            }
            let result_count = self.binding_result_word_count(function)?;
            self.observe(program::Event::Binding {
                event: program::BindingEvent::Enter,
                binding_id: binding.id,
            })?;
            Self::call_binding(
                current,
                &self.fiber.stack,
                &mut self.machine.binding_buffer,
                &mut self.activation,
                self.fiber.fiber_id,
                &binding,
                environment,
                arguments,
                result_count,
            )?;
            self.observe(program::Event::Binding {
                event: program::BindingEvent::Exit,
                binding_id: binding.id,
            })?;
            let Ok(result_word_count) = u16::try_from(result_count) else {
                return Err(self.invalid_instruction().into());
            };
            let byte_len = result_count * Word::BYTE_LEN;
            self.fiber.stack.grow(current.byte_offset() + byte_len)?;
            for index in 0..result_count {
                let value = self.machine.binding_buffer[index];
                self.fiber
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
        self.fiber.stack.grow(end)?;

        // move arguments before replacing bytes that may overlap their source
        let argument_offset = register_offset + environment_word_count;
        self.fiber.stack.move_words(
            argument_start,
            argument_offset,
            arguments.word_count as usize,
        );

        // initialize the hidden environment after moving overlapping arguments
        if let Some(environment) = environment {
            self.fiber.stack.write(register_offset, environment);
        }
        self.fiber.stack.truncate(end);

        // leave the replaced frame before entering its tail-called successor
        self.observe(program::Event::Frame {
            event: program::FrameEvent::Exit,
            function: current.function,
        })?;

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
        self.observe(program::Event::Frame {
            event: program::FrameEvent::Enter,
            function,
        })?;

        Ok(None)
    }

    /// Call one runtime binding with flattened frame arguments.
    fn call_binding(
        frame: Frame,
        stack: &super::Stack,
        words: &mut Vec<Word>,
        activation: &mut program::Activation<'run, 'run, R>,
        fiber_id: Option<program::FiberId>,
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
            .call_binding(
                memory,
                *activation.context,
                fiber_id,
                binding,
                arguments,
                result,
            )
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
        let Some(frame) = self.fiber.frames.pop() else {
            unreachable!("bytecode returns require an active frame");
        };
        let result_start = frame.range(results);
        self.observe(program::Event::Frame {
            event: program::FrameEvent::Exit,
            function: frame.function,
        })?;
        if !self.fiber.frames.is_empty() {
            self.activate();
        }

        match frame.return_to {
            // publish the root frame outcome
            Return::Exit { completion } => {
                let values = self
                    .fiber
                    .stack
                    .words(result_start, results.word_count as usize);
                self.fiber.stack.truncate(frame.byte_offset());
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
                let Some(caller) = self.fiber.frames.last().copied() else {
                    unreachable!("called frames require a caller");
                };
                let target = caller.range(registers);
                self.fiber
                    .stack
                    .copy_words(result_start, target, results.word_count as usize);
                if let Some(normal) = normal {
                    self.jump(normal);
                }
                self.fiber.stack.truncate(frame.byte_offset());

                Ok(None)
            }

            // release one destructor frame or its retained continuation frames
            Return::Drop { frame_count, .. } => {
                if results.word_count != 0 {
                    return Err(self.invalid_instruction().into());
                }
                self.finish_destructor(frame, frame_count)?;

                Ok(None)
            }

            // destroy the next released value, or free the allocation after its last
            Return::Release(released) => {
                if results.word_count != 0 {
                    return Err(self.invalid_instruction().into());
                }
                self.finish_destructor(frame, released.frame_count)?;
                self.destroy_released(released)?;

                Ok(None)
            }
        }
    }

    /// Release one completed destructor frame and its retained continuation frames.
    fn finish_destructor(
        &mut self,
        frame: Frame,
        frame_count: u16,
    ) -> ExecutionResult<(), R::Error> {
        let stack_byte_len = if frame_count == 0 {
            frame.byte_offset()
        } else {
            let first_frame = self
                .fiber
                .frames
                .len()
                .checked_sub(frame_count as usize)
                .ok_or_else(|| self.invalid_instruction())?;
            let Some(first) = self.fiber.frames.get(first_frame).copied() else {
                return Err(self.invalid_instruction().into());
            };

            // publish and remove retained frames from innermost to outermost
            while self.fiber.frames.len() > first_frame {
                let Some(frame) = self.fiber.frames.pop() else {
                    return Err(self.invalid_instruction().into());
                };
                self.observe(program::Event::Frame {
                    event: program::FrameEvent::Exit,
                    function: frame.function,
                })?;
            }

            first.byte_offset()
        };
        self.fiber.stack.truncate(stack_byte_len);
        if !self.fiber.frames.is_empty() {
            self.activate();
        }

        Ok(())
    }

    /// Return the active frame by value.
    #[inline(always)]
    pub(crate) fn frame(&self) -> Frame {
        self.cursor.frame()
    }

    /// Return the mounted logical fiber identity.
    pub(crate) fn fiber_id(&self) -> Result<program::FiberId> {
        self.fiber
            .fiber_id
            .ok_or_else(|| self.invalid_instruction())
    }

    /// Save the live instruction offset in the active canonical frame.
    #[inline(always)]
    pub(crate) fn save_position(&mut self) {
        self.cursor.save_position();
    }

    /// Retain stopped execution in place with the exact resume position.
    pub(crate) fn retain_stop(&mut self, pc: CodeOffset) {
        let Some(frame) = self.fiber.frames.last_mut() else {
            unreachable!("stops require an active frame");
        };

        frame.pc = pc;
        self.fiber.context = *self.activation.context;
    }

    /// Derive native execution addresses for the active canonical frame.
    pub(crate) fn activate(&mut self) {
        let Some(frame) = self.fiber.frames.last_mut() else {
            unreachable!("bytecode execution requires an active frame");
        };
        let active = *frame;
        let frame = ptr::from_mut(frame);
        let sections = self.machine.program.sections();
        let bytes = self.machine.bytecode.bytes(sections);

        // materialize native addresses only for this activation
        let code = unsafe { bytes.as_ptr().add(active.code.byte_offset as usize) };
        let registers = self.fiber.stack.address(active.byte_offset()) as *mut Word;

        // SAFETY: linked frame ranges address Program code and live stack registers
        unsafe {
            self.cursor.set(code, active.pc, registers, frame);
        }
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
        let Some(last) = self.fiber.frames.len().checked_sub(1) else {
            unreachable!("located execution failures require an active frame");
        };

        // capture each active bytecode location from entry to failure
        let stack = self
            .fiber
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
            .fiber
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
