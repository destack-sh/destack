use destack_bytecode::{CodeOffset, Instruction, Opcode};
use destack_program as program;
use destack_program::{
    CallMode, Completion, Continuation, ContinuationTable, FrameImage, FrameLink, FramePoint,
    FrameStateId, FunctionId, Outcome, Profile, ProgramPoint, Runtime, StopSet, SuspensionSite,
    Value, WatchSet, Word,
};

use crate::diagnostic::{Error, ExecutionError, Result};

use super::{Activation, Frame, FrameMapping, Machine, Return};

impl Machine {
    /// Cancel one suspended asynchronous continuation through its cleanup path.
    pub fn cancel<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        continuation: Continuation,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() || self.activation.is_some() {
            return Err(Error::execution_active().into());
        }

        // validate and restore before releasing the consumed canonical range
        let function = self.consume_continuation(continuation, |machine, continuation| {
            let (function, site) = machine.suspension(continuation)?;
            if site.operation != program::Suspension::Await {
                return Err(Error::invalid_image());
            }
            let return_to = Return::Exit {
                completion: Completion::Cancel,
            };
            machine.materialize_continuation(continuation, return_to)?;

            Ok(function)
        })?;

        // execute the restored cancellation path
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, None)
            .cancel()
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Resume one suspended coroutine with one value.
    pub fn resume<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        continuation: Continuation,
        value: &Value,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() || self.activation.is_some() {
            return Err(Error::execution_active().into());
        }

        // validate and restore before releasing the consumed canonical range
        let (function, values) =
            self.consume_continuation(continuation, |machine, continuation| {
                let (function, site) = machine.suspension(continuation)?;
                let values = machine
                    .program
                    .value_words(site.resume_type, value)
                    .map_err(Error::program)?
                    .to_vec();
                let return_to = Return::Exit {
                    completion: continuation.completion(),
                };
                machine.materialize_continuation(continuation, return_to)?;

                Ok((function, values))
            })?;

        // execute the restored resume path
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, None)
            .resume(&values)
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Complete one suspended generator with one value.
    pub fn complete<'run, R>(
        &mut self,
        continuations: &mut ContinuationTable,
        activation: program::Activation<'run, 'run, R>,
        continuation: Continuation,
        value: &Value,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut Profile>,
    ) -> std::result::Result<Outcome<Value>, R::Error>
    where
        R: Runtime + ?Sized,
        R::Error: From<Error>,
    {
        if !self.frames.is_empty() || self.activation.is_some() {
            return Err(Error::execution_active().into());
        }

        // validate and restore before releasing the consumed canonical range
        let (function, values) =
            self.consume_continuation(continuation, |machine, continuation| {
                let (function, site) = machine.suspension(continuation)?;
                if site.operation != program::Suspension::Yield {
                    return Err(Error::invalid_image());
                }
                let ty = site.complete_type.get().ok_or_else(Error::invalid_image)?;
                let values = machine
                    .program
                    .value_words(ty, value)
                    .map_err(Error::program)?
                    .to_vec();
                let return_to = Return::Exit {
                    completion: continuation.completion(),
                };
                machine.materialize_continuation(continuation, return_to)?;

                Ok((function, values))
            })?;

        // execute the restored completion path
        let outcome = Activation::new(self, continuations, activation)
            .instrument(stop_points, watch_points, profile, None)
            .complete(&values)
            .map_err(ExecutionError::into_error)?;

        self.decode_outcome(function, outcome).map_err(Into::into)
    }

    /// Capture one coroutine entry frame before its first instruction.
    pub(crate) fn create_continuation(
        &mut self,
        function: FunctionId,
        arguments: &[Word],
    ) -> Result<Continuation> {
        let point = FramePoint::entry(function);
        let state = self
            .program
            .frame_state_at(point)
            .ok_or_else(Error::invalid_image)?;
        let first_frame = self.frames.len();
        let byte_len = self.stack.byte_len();
        let return_to = Return::Exit {
            completion: Completion::Return,
        };
        let frame = self.allocate_frame(function, arguments.len(), return_to)?;

        // initialize the complete captured argument prefix
        for (index, argument) in arguments.iter().copied().enumerate() {
            self.stack.write(frame.register_offset + index, argument);
        }
        self.frames.push(frame);

        // canonicalize the entry frame before releasing physical storage
        let continuation = self.capture_continuation(first_frame, state, Completion::Return);
        self.frames.truncate(first_frame);
        self.stack.truncate(byte_len);

        continuation
    }

    /// Capture and release the active coroutine call chain.
    pub(crate) fn suspend(&mut self, state: FrameStateId) -> Result<Continuation> {
        let completion = match self.frames.first().map(|frame| frame.return_to) {
            Some(Return::Exit { completion }) => completion,
            _ => return Err(Error::invalid_image()),
        };
        let continuation = self.capture_continuation(0, state, completion)?;
        self.release_frames(0)?;

        Ok(continuation)
    }

    /// Capture and release one active coroutine subtree.
    pub(crate) fn suspend_suffix(
        &mut self,
        first_frame: usize,
        state: FrameStateId,
    ) -> Result<Continuation> {
        let continuation = self.capture_suffix(first_frame, state)?;
        self.release_frames(first_frame)?;

        Ok(continuation)
    }

    /// Capture one active call-chain suffix without releasing its physical frames.
    pub(crate) fn capture_suffix(
        &self,
        first_frame: usize,
        state: FrameStateId,
    ) -> Result<Continuation> {
        self.capture_continuation(first_frame, state, Completion::Return)
    }

    /// Release one active frame suffix after its canonical capture succeeds.
    pub(crate) fn release_frames(&mut self, first_frame: usize) -> Result<()> {
        let byte_offset = self
            .frames
            .get(first_frame)
            .copied()
            .map(Frame::byte_offset)
            .ok_or_else(Error::invalid_image)?;
        self.frames.truncate(first_frame);
        self.stack.truncate(byte_offset);

        Ok(())
    }

    /// Capture one active call-chain suffix.
    fn capture_continuation(
        &self,
        first_frame: usize,
        state: FrameStateId,
        completion: Completion,
    ) -> Result<Continuation> {
        let mappings = self.frame_mappings(first_frame, state)?;
        let frames = mappings
            .iter()
            .copied()
            .map(|mapping| self.frame_image(mapping))
            .collect::<Result<Vec<_>>>()?;
        let bytes = self.pack_frames(&mappings)?;
        let memory = self.store_image(&bytes)?;

        Ok(Continuation::new(completion, frames, memory))
    }

    /// Return the innermost logical coordinate retained by one continuation.
    pub(crate) fn continuation_point(&self, continuation: &Continuation) -> Result<FramePoint> {
        let state = continuation
            .innermost()
            .map(FrameImage::state)
            .ok_or_else(Error::invalid_image)?;

        self.program
            .frame_point(state)
            .ok_or_else(Error::invalid_image)
    }

    /// Materialize one suspended call chain with its root transition.
    pub(crate) fn materialize_continuation(
        &mut self,
        continuation: &Continuation,
        return_to: Return,
    ) -> Result<()> {
        let first_frame = self.frames.len();
        let byte_len = self.stack.byte_len();
        let result = self
            .materialize_frames(continuation.frames(), return_to)
            .and_then(|frames| {
                let mut bytes = self.load_image(continuation.memory())?;

                self.unpack_frames(&frames, &mut bytes)
            });
        if result.is_err() {
            self.frames.truncate(first_frame);
            self.stack.truncate(byte_len);
        }

        result
    }

    /// Apply one transition and release its consumed continuation.
    pub(crate) fn consume_continuation<T>(
        &mut self,
        continuation: Continuation,
        transition: impl FnOnce(&mut Self, &Continuation) -> Result<T>,
    ) -> Result<T> {
        let result = transition(self, &continuation);
        let release = continuation
            .release(&self.stack.memory())
            .map_err(Error::program);
        if release.is_err() {
            self.clear_physical();
        }
        release?;

        result
    }

    /// Restore one suspended coroutine above an active caller frame.
    pub(crate) fn attach_continuation(
        &mut self,
        continuation: Continuation,
        return_to: Return,
    ) -> Result<()> {
        self.consume_continuation(continuation, |machine, continuation| {
            machine.materialize_continuation(continuation, return_to)
        })
    }

    /// Materialize canonical frames with one physical root transition.
    pub(crate) fn materialize_frames(
        &mut self,
        images: &[FrameImage],
        root_return: Return,
    ) -> Result<Vec<FrameMapping>> {
        if images.is_empty() {
            return Err(Error::invalid_image());
        }
        let mut frames = Vec::with_capacity(images.len());
        let mut byte_offset = 0usize;

        // rebuild each physical frame from canonical Program state
        for (index, image) in images.iter().copied().enumerate() {
            let state = image.state();
            let linked = self
                .program
                .frame_state(state)
                .copied()
                .ok_or_else(Error::invalid_image)?;
            let function = linked.point.function();
            let pc = self.pc(image.point())?;

            // rebuild the child return from its caller's canonical call site
            let return_to = if index == 0 {
                root_return
            } else {
                self.frame_return(images[index - 1], image.link())?
            };

            // resolve the physical bytecode frame
            let linked = self
                .bytecode
                .function(self.program.sections(), function.index())
                .ok_or_else(|| Error::undefined_function(function))?;
            let code = linked
                .code()
                .ok_or_else(|| Error::undefined_function(function))?;

            // allocate physical registers at the canonical frame alignment
            let layout = self.frame_layout(state)?;
            byte_offset = byte_offset.next_multiple_of(layout.alignment() as usize);
            let register_offset = self.stack.push_words(linked.register_count())?;
            let mut frame = Frame::new(
                function,
                code,
                register_offset,
                linked.register_count,
                return_to,
            );
            frame.pc = pc;

            // retain the frame and its canonical live value mapping
            self.frames.push(frame);
            frames.push(self.frame_mapping(frame, state, byte_offset)?);
            byte_offset += layout.byte_len() as usize;
        }

        Ok(frames)
    }

    /// Rebuild one child return from its caller's canonical call state.
    pub(crate) fn frame_return(&self, parent: FrameImage, link: FrameLink) -> Result<Return> {
        if link == FrameLink::Root {
            return Err(Error::invalid_image());
        }
        if let FrameLink::Drop { frame_count } = link {
            let pc = self.pc(parent.point())?;

            return Ok(Return::Drop {
                pc,
                caller_state: parent.state(),
                frame_count,
            });
        }
        let state = self
            .program
            .frame_state(parent.state())
            .ok_or_else(Error::invalid_image)?;
        let point = state
            .point
            .operation_point()
            .ok_or_else(Error::invalid_image)?;
        let instruction = self
            .bytecode
            .operation(
                self.program.sections(),
                point.function.index(),
                point.operation,
            )
            .map_err(|_| Error::invalid_image())?
            .ok_or_else(Error::invalid_image)?;
        match link {
            FrameLink::Call if Self::is_returning_call(instruction.opcode()) => {
                self.continuation_call_return(point, instruction)
            }
            FrameLink::Continuation
                if matches!(
                    instruction.opcode(),
                    Opcode::CONTINUATION_RESUME | Opcode::CONTINUATION_COMPLETE
                ) =>
            {
                self.continuation_return(point, instruction)
            }
            FrameLink::Task if instruction.opcode() == Opcode::TASK_START => {
                let mut operands = instruction.operands();
                let task_register = operands.register().map_err(|_| Error::invalid_image())?.0;

                Ok(Return::Task {
                    pc: self.pc(point)?,
                    task_register,
                })
            }
            FrameLink::Root | FrameLink::Call | FrameLink::Continuation | FrameLink::Task => {
                Err(Error::invalid_image())
            }
            FrameLink::Drop { .. } => unreachable!("drop links return before decoding callers"),
        }
    }

    /// Rebuild one ordinary caller transition.
    fn continuation_call_return(
        &self,
        point: ProgramPoint,
        instruction: Instruction<'_>,
    ) -> Result<Return> {
        let (_, site) = self.program.call(point).ok_or_else(Error::invalid_image)?;
        if site.mode != CallMode::Return {
            return Err(Error::invalid_image());
        }
        let mut operands = instruction.operands();
        let registers = operands.span().map_err(|_| Error::invalid_image())?;
        let pc = self.pc(point)?;
        let resume = site.resume.get().ok_or_else(Error::invalid_image)?;
        let normal = Some(self.call_offset(point.function, resume)?);
        let unwind = site
            .unwind
            .get()
            .map(|point| self.call_offset(point.function, point))
            .transpose()?;

        Ok(Return::Call {
            pc,
            registers,
            normal,
            unwind,
        })
    }

    /// Rebuild one continuation control transition.
    fn continuation_return(
        &self,
        point: ProgramPoint,
        instruction: Instruction<'_>,
    ) -> Result<Return> {
        let (_, site) = self
            .program
            .continuation(point)
            .ok_or_else(Error::invalid_image)?;
        let mut operands = instruction.operands();
        let yielded_registers = operands.span().map_err(|_| Error::invalid_image())?;
        let continuation_register = operands.register().map_err(|_| Error::invalid_image())?.0;
        let returned_registers = operands.span().map_err(|_| Error::invalid_image())?;
        let pc = self.pc(point)?;
        let yielded = self.call_offset(point.function, site.yielded)?;
        let returned = self.call_offset(point.function, site.returned)?;
        let unwind = site
            .unwind
            .get()
            .map(|point| self.call_offset(point.function, point))
            .transpose()?;

        Ok(Return::Continuation {
            pc,
            yielded_registers,
            continuation_register,
            returned_registers,
            yielded,
            returned,
            unwind,
        })
    }

    /// Return whether one opcode enters a callee that returns to its caller.
    const fn is_returning_call(opcode: Opcode) -> bool {
        matches!(
            opcode,
            Opcode::CALL
                | Opcode::CALL_INDIRECT
                | Opcode::CALL_VIRTUAL
                | Opcode::CALL_DYNAMIC
                | Opcode::INVOKE
                | Opcode::INVOKE_INDIRECT
                | Opcode::INVOKE_VIRTUAL
                | Opcode::INVOKE_DYNAMIC
        )
    }

    /// Resolve one call destination inside its caller function.
    fn call_offset(&self, function: FunctionId, point: ProgramPoint) -> Result<CodeOffset> {
        if point.function != function {
            return Err(Error::invalid_image());
        }

        self.pc(point)
    }

    /// Resolve one canonical program point into a bytecode offset.
    pub(crate) fn pc(&self, point: ProgramPoint) -> Result<CodeOffset> {
        self.bytecode
            .operation_offset(
                self.program.sections(),
                point.function.index(),
                point.operation,
            )
            .ok_or_else(Error::invalid_image)
    }

    /// Resolve the linked site captured by one continuation.
    pub(crate) fn suspension(
        &self,
        continuation: &Continuation,
    ) -> Result<(FunctionId, SuspensionSite)> {
        let root_state = continuation
            .frames()
            .first()
            .copied()
            .map(FrameImage::state)
            .ok_or_else(Error::invalid_image)?;
        let root = self
            .program
            .frame_state(root_state)
            .ok_or_else(Error::invalid_image)?;
        let suspension_state = continuation
            .innermost()
            .map(FrameImage::state)
            .ok_or_else(Error::invalid_image)?;
        let suspension = self
            .program
            .frame_state(suspension_state)
            .ok_or_else(Error::invalid_image)?;
        let point = suspension
            .point
            .operation_point()
            .ok_or_else(Error::invalid_image)?;
        let (_, site) = self
            .program
            .suspension(point)
            .ok_or_else(Error::invalid_image)?;
        if site.frame_state != suspension_state {
            return Err(Error::invalid_image());
        }

        Ok((root.point.function(), *site))
    }
}
