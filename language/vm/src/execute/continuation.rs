use std::ops::Range;

use destack_bytecode::{CodeOffset, FrameSlot, Instruction, Opcode, RegisterRange};
use destack_program::{
    Continuation, ContinuationBuilder, ContinuationFrame, FrameLayout, FrameSlotId, FrameStateId,
    Outcome, Program, Word,
};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Frame, Return};

impl Activation<'_, '_> {
    /// Continue one canonical continuation at its captured program point.
    pub(crate) fn continue_execution(
        mut self,
        continuation: Continuation,
    ) -> Result<Outcome<Continuation, Vec<Word>>> {
        self.restore_frames(&continuation)?;
        let Some(saved) = continuation.frames.last().copied() else {
            unreachable!("restored continuations contain an innermost frame");
        };
        let state = self
            .machine
            .program
            .frame_state(saved.frame_state)
            .ok_or_else(Error::invalid_continuation)?;
        let code_offset = self.code_offset(state.point)?;
        self.frame_mut().code_offset = code_offset;

        self.execute()
    }

    /// Restore and resume one canonical continuation.
    pub(crate) fn resume(
        mut self,
        continuation: Continuation,
        received: &[Word],
    ) -> Result<Outcome<Continuation, Vec<Word>>> {
        self.restore_frames(&continuation)?;
        let Some(resume_frame) = continuation.frames.last().copied() else {
            unreachable!("restored continuations contain an innermost frame");
        };

        // bind the received words into the innermost frame
        let frame = self.frame();
        let received_range = self.received_range(resume_frame.frame_state)?;
        if received_range.word_count as usize != received.len() {
            return Err(Error::invalid_continuation());
        }
        for (index, value) in received.iter().copied().enumerate() {
            self.machine
                .stack
                .write(frame.range(received_range) + index, value);
        }

        // enter the engine-neutral Program resume point
        let program = &self.machine.program;
        let (site_id, site) = program
            .sites()
            .continuation_state(program.sections(), resume_frame.frame_state)
            .ok_or_else(Error::invalid_continuation)?;
        if site.resume.function != frame.function {
            return Err(Error::invalid_continuation());
        }
        let code_offset = self.code_offset(site.resume)?;
        self.frame_mut().code_offset = code_offset;

        // record the resumed continuation only when profiling is active
        if let Some(profile) = self.profile.as_deref_mut() {
            profile.record_continuation_resume(site_id);
        }

        self.execute()
    }

    /// Suspend at one bytecode yield and return its canonical continuation.
    pub(crate) fn execute_yield(
        &mut self,
        instruction: Instruction<'_>,
        instruction_offset: CodeOffset,
    ) -> Result<Outcome<Continuation, Vec<Word>>> {
        if let Some(function) = self.destructor() {
            return Err(Error::invalid_destructor(function));
        }

        let mut operands = instruction.operands();
        let _received = operands.range().map_err(|_| self.invalid_instruction())?;
        let yielded = operands.range().map_err(|_| self.invalid_instruction())?;
        let _yielded_type = operands
            .value_type()
            .map_err(|_| self.invalid_instruction())?;
        let _resume = operands.i32().map_err(|_| self.invalid_instruction())?;
        let _unwind = operands.i32().map_err(|_| self.invalid_instruction())?;
        let frame = self.frame();
        let point = self.point(frame, instruction_offset)?;
        let frame_state = self
            .machine
            .program
            .frame_state_at(point)
            .ok_or_else(Error::invalid_continuation)?;
        let value = self
            .machine
            .stack
            .words(frame.range(yielded), yielded.word_count as usize);
        let continuation = self.capture(frame_state)?;

        Ok(Outcome::Yielded {
            continuation,
            value,
        })
    }

    /// Capture every active frame into canonical Program frame bytes.
    pub(crate) fn capture(&mut self, current_state: FrameStateId) -> Result<Continuation> {
        let mut continuation = ContinuationBuilder::new();
        let Some(last) = self.machine.frames.len().checked_sub(1) else {
            unreachable!("continuation capture requires an active frame");
        };

        // capture frames from the entry frame through the suspension frame
        for (index, frame) in self.machine.frames.iter().copied().enumerate() {
            let frame_state = if index == last {
                current_state
            } else {
                frame.frame_state.ok_or_else(Error::invalid_continuation)?
            };
            let bytes = self.frame_bytes(frame, frame_state)?;
            continuation.push(frame_state, frame.normal_state, frame.unwind_state, &bytes);
        }

        // record this managed suspension when profiling is active
        if let Some((site_id, _)) = self
            .machine
            .program
            .sites()
            .continuation_state(self.machine.program.sections(), current_state)
            && let Some(profile) = self.profile.as_deref_mut()
        {
            profile.record_continuation_capture(site_id);
        }

        Ok(continuation.build())
    }

    /// Restore every canonical continuation frame into ephemeral VM storage.
    fn restore_frames(&mut self, continuation: &Continuation) -> Result<()> {
        if continuation.frames.is_empty() {
            return Err(Error::invalid_continuation());
        }
        let program = self.machine.program.clone();

        // rebuild frames from entry to suspension with caller return destinations
        for (index, saved) in continuation.frames.iter().copied().enumerate() {
            let return_to = if index == 0 {
                None
            } else {
                let caller = continuation.frames[index - 1];
                Some(Return::Values(self.call_results(caller.frame_state)?))
            };
            let mut frame = self.restore_frame(&program, continuation, saved, return_to)?;
            frame.suspend(saved.frame_state, saved.normal_state, saved.unwind_state);

            // outer frames wait at their normal return destination
            if index + 1 < continuation.frames.len() {
                frame.code_offset = self.return_offset(saved)?;
            }
            self.machine.frames.push(frame);
        }

        Ok(())
    }

    /// Restore one canonical continuation frame into ephemeral VM storage.
    fn restore_frame(
        &mut self,
        program: &Program,
        continuation: &Continuation,
        saved: ContinuationFrame,
        return_to: Option<Return>,
    ) -> Result<Frame> {
        let state = program
            .frame_state(saved.frame_state)
            .copied()
            .ok_or_else(Error::invalid_continuation)?;
        let function = state.point.function;
        let frame = self.allocate_frame(function, 0, return_to)?;
        let bytes = continuation
            .frame_bytes(&saved)
            .ok_or_else(Error::invalid_continuation)?;
        if bytes.len() != frame.frame_byte_len as usize {
            return Err(Error::invalid_continuation());
        }
        self.machine.stack.write_bytes(frame.frame_offset, bytes);

        // restore only live register-backed slots from canonical bytes
        let body = self
            .body(function)
            .map_err(|_| Error::invalid_continuation())?;
        let slots = body.frame_slots(program.bytecode().frame_slots(program.sections()));
        let layout = program
            .frame_layout_by_id(state.frame_layout)
            .copied()
            .ok_or_else(Error::invalid_continuation)?;
        for slot_id in program.frame_live_slots(&state) {
            self.restore_frame_slot(program, frame, &layout, slots, *slot_id, bytes)?;
        }

        Ok(frame)
    }

    /// Return the caller result registers encoded by one captured call.
    fn call_results(&self, frame_state: FrameStateId) -> Result<RegisterRange> {
        let instruction = self.frame_instruction(frame_state)?;
        if !matches!(
            instruction.opcode(),
            Opcode::CALL
                | Opcode::CALL_INDIRECT
                | Opcode::CALL_VIRTUAL
                | Opcode::CALL_DYNAMIC
                | Opcode::INVOKE
                | Opcode::INVOKE_INDIRECT
                | Opcode::INVOKE_VIRTUAL
                | Opcode::INVOKE_DYNAMIC
        ) {
            return Err(Error::invalid_continuation());
        }
        let mut operands = instruction.operands();

        operands.range().map_err(|_| Error::invalid_continuation())
    }

    /// Return the normal bytecode destination retained by one caller frame.
    fn return_offset(&self, saved: ContinuationFrame) -> Result<CodeOffset> {
        if let Some(normal_state) = saved.normal_state {
            let point = self
                .machine
                .program
                .frame_point(normal_state)
                .ok_or_else(Error::invalid_continuation)?;

            return self.code_offset(point);
        }

        let state = self
            .machine
            .program
            .frame_state(saved.frame_state)
            .ok_or_else(Error::invalid_continuation)?;
        let instruction = self.frame_instruction(saved.frame_state)?;
        let offset = self.code_offset(state.point)?;

        Ok(CodeOffset(offset.0 + instruction.byte_len() as u32))
    }

    /// Decode the bytecode instruction represented by one frame state.
    fn frame_instruction(&self, frame_state: FrameStateId) -> Result<Instruction<'_>> {
        let program = &self.machine.program;
        let state = program
            .frame_state(frame_state)
            .ok_or_else(Error::invalid_continuation)?;

        program
            .bytecode()
            .instruction(
                program.sections(),
                state.point.function.index(),
                state.point.operation,
            )
            .map_err(|_| Error::invalid_continuation())?
            .ok_or_else(Error::invalid_continuation)
    }

    /// Restore one canonical frame slot into its fixed register range.
    fn restore_frame_slot(
        &mut self,
        program: &Program,
        frame: Frame,
        layout: &FrameLayout,
        slots: &[FrameSlot],
        slot_id: FrameSlotId,
        bytes: &[u8],
    ) -> Result<()> {
        let Some((registers, bytes_range)) =
            self.resolve_slot(program, layout, slots, slot_id, bytes.len())?
        else {
            return Ok(());
        };
        let register_offset = frame.range(registers) * Word::BYTE_LEN;
        let register_byte_len = registers.word_count as usize * Word::BYTE_LEN;
        self.machine
            .stack
            .zero(register_offset, register_byte_len)?;
        self.machine
            .stack
            .write_bytes(register_offset, &bytes[bytes_range]);

        Ok(())
    }

    /// Return the result register range encoded by one captured yield.
    fn received_range(&self, frame_state: FrameStateId) -> Result<RegisterRange> {
        let instruction = self.frame_instruction(frame_state)?;
        if instruction.opcode() != Opcode::YIELD {
            return Err(Error::invalid_continuation());
        }
        let mut operands = instruction.operands();

        operands.range().map_err(|_| Error::invalid_continuation())
    }

    /// Materialize one active frame into its canonical Program byte layout.
    fn frame_bytes(&self, frame: Frame, frame_state: FrameStateId) -> Result<Vec<u8>> {
        let program = &self.machine.program;
        let state = program
            .frame_state(frame_state)
            .ok_or_else(Error::invalid_continuation)?;
        let layout = program
            .frame_layout_by_id(state.frame_layout)
            .ok_or_else(Error::invalid_continuation)?;
        if layout.byte_len() != frame.frame_byte_len {
            return Err(Error::invalid_continuation());
        }
        let body = self
            .body(frame.function)
            .map_err(|_| Error::invalid_continuation())?;
        let slots = body.frame_slots(program.bytecode().frame_slots(program.sections()));
        let mut bytes = vec![0; frame.frame_byte_len as usize];
        self.machine
            .stack
            .read_bytes(frame.frame_offset, &mut bytes);

        // overlay each live register-backed slot onto canonical frame storage
        for slot_id in program.frame_live_slots(state) {
            self.capture_frame_slot(frame, layout, slots, *slot_id, &mut bytes)?;
        }

        Ok(bytes)
    }

    /// Capture one register-backed slot into canonical frame bytes.
    fn capture_frame_slot(
        &self,
        frame: Frame,
        layout: &FrameLayout,
        slots: &[FrameSlot],
        slot_id: FrameSlotId,
        bytes: &mut [u8],
    ) -> Result<()> {
        let Some((registers, bytes_range)) =
            self.resolve_slot(&self.machine.program, layout, slots, slot_id, bytes.len())?
        else {
            return Ok(());
        };
        let register_offset = frame.range(registers) * Word::BYTE_LEN;
        self.machine
            .stack
            .read_bytes(register_offset, &mut bytes[bytes_range]);

        Ok(())
    }

    /// Resolve one canonical frame slot and its register range.
    fn resolve_slot(
        &self,
        program: &Program,
        layout: &FrameLayout,
        slots: &[FrameSlot],
        slot_id: FrameSlotId,
        frame_byte_len: usize,
    ) -> Result<Option<(RegisterRange, Range<usize>)>> {
        let Some(registers) = slots
            .get(slot_id.0 as usize)
            .and_then(|slot| slot.registers())
        else {
            return Ok(None);
        };
        let slot = program
            .frame_slot(layout, slot_id)
            .ok_or_else(Error::invalid_continuation)?;
        let start = slot.offset as usize;
        let end = start + slot.byte_len() as usize;
        let register_byte_len = registers.word_count as usize * Word::BYTE_LEN;
        if end > frame_byte_len || slot.byte_len() as usize > register_byte_len {
            return Err(Error::invalid_continuation());
        }

        Ok(Some((registers, start..end)))
    }
}
