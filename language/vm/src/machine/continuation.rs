use destack_bytecode::{CodeOffset, Instruction, Opcode};
use destack_program::{
    CallMode, Completion, Continuation, FramePoint, FrameStateId, FunctionId, ProgramPoint, Word,
};

use crate::diagnostic::{Error, Result};

use super::{Frame, FrameMapping, Machine, Return};

impl Machine {
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
    pub(crate) fn suspend_from(
        &mut self,
        first_frame: usize,
        state: FrameStateId,
    ) -> Result<Continuation> {
        let continuation = self.capture_from(first_frame, state)?;
        self.release_frames(first_frame)?;

        Ok(continuation)
    }

    /// Capture one active call-chain suffix without releasing its physical frames.
    pub(crate) fn capture_from(
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
        let frames = self.active_frame_mappings_from(first_frame, state)?;
        let states = frames.iter().map(|frame| frame.state).collect::<Vec<_>>();
        let bytes = self.pack_frames(&frames)?;

        Ok(Continuation::new(completion, states, bytes))
    }

    /// Return the innermost logical coordinate retained by one continuation.
    pub(crate) fn continuation_point(&self, continuation: &Continuation) -> Result<FramePoint> {
        let state = continuation.innermost().ok_or_else(Error::invalid_image)?;

        self.program
            .frame_point(state)
            .ok_or_else(Error::invalid_image)
    }

    /// Restore one suspended coroutine call chain.
    pub(crate) fn restore_continuation(&mut self, continuation: &Continuation) -> Result<()> {
        let return_to = Return::Exit {
            completion: continuation.completion(),
        };

        self.restore_continuation_with(continuation, return_to)
    }

    /// Restore one suspended asynchronous call chain for cancellation.
    pub(crate) fn restore_continuation_for_cancel(
        &mut self,
        continuation: &Continuation,
    ) -> Result<()> {
        let return_to = Return::Exit {
            completion: Completion::Cancel,
        };

        self.restore_continuation_with(continuation, return_to)
    }

    /// Restore one suspended call chain with its root transition.
    fn restore_continuation_with(
        &mut self,
        continuation: &Continuation,
        return_to: Return,
    ) -> Result<()> {
        if !self.frames.is_empty() {
            return Err(Error::execution_active());
        }

        // rebuild physical return transitions from canonical Program call sites
        let result = self
            .restore_continuation_frames(continuation.states(), return_to)
            .and_then(|frames| self.unpack_frames(&frames, continuation.bytes()));
        if result.is_err() {
            self.clear();
        }

        result
    }

    /// Restore one suspended coroutine above an active caller frame.
    pub(crate) fn attach_continuation(
        &mut self,
        continuation: &Continuation,
        return_to: Return,
    ) -> Result<()> {
        let first_frame = self.frames.len();
        let byte_offset = self
            .frames
            .last()
            .map(|frame| frame.byte_offset() + frame.register_count as usize * Word::BYTE_LEN)
            .ok_or_else(Error::invalid_image)?;
        let result = self
            .restore_continuation_frames(continuation.states(), return_to)
            .and_then(|frames| self.unpack_frames(&frames, continuation.bytes()));
        if result.is_err() {
            self.frames.truncate(first_frame);
            self.stack.truncate(byte_offset);
        }

        result
    }

    /// Restore physical VM frames from canonical continuation states.
    fn restore_continuation_frames(
        &mut self,
        states: &[FrameStateId],
        root_return: Return,
    ) -> Result<Vec<FrameMapping>> {
        if states.is_empty() {
            return Err(Error::invalid_image());
        }
        let mut frames = Vec::with_capacity(states.len());
        let mut byte_offset = 0usize;

        // rebuild each physical frame from canonical Program state
        for (index, state) in states.iter().copied().enumerate() {
            let linked = self
                .program
                .frame_state(state)
                .copied()
                .ok_or_else(Error::invalid_image)?;
            let function = linked.point.function();
            let pc = match linked.point {
                FramePoint::Entry { .. } => CodeOffset(0),
                FramePoint::Operation(point) => self.pc(point)?,
            };

            // rebuild the child return from its caller's canonical call site
            let return_to = if index == 0 {
                root_return
            } else {
                self.frame_return(states[index - 1])?
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
    fn frame_return(&self, state: FrameStateId) -> Result<Return> {
        let state = self
            .program
            .frame_state(state)
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
        if Self::is_returning_call(instruction.opcode()) {
            self.continuation_call_return(point, instruction)
        } else if matches!(
            instruction.opcode(),
            Opcode::CONTINUATION_RESUME | Opcode::CONTINUATION_COMPLETE
        ) {
            self.continuation_return(point, instruction)
        } else {
            Err(Error::invalid_image())
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
    fn pc(&self, point: ProgramPoint) -> Result<CodeOffset> {
        self.bytecode
            .operation_offset(
                self.program.sections(),
                point.function.index(),
                point.operation,
            )
            .ok_or_else(Error::invalid_image)
    }
}
