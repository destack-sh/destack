use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use destack_heap::{HeapResult, RootSlot};
use destack_program::{
    Continuation, ContinuationFrame, FrameLayout, FrameMaterialization, FrameSlot, FrameStateId,
    Program,
};

use super::{Frame, Machine, visit_frame_slot_root_slots, visit_materialized_slots};

impl Machine {
    /// Capture active execution into one continuation.
    pub(crate) fn capture_continuation(
        &mut self,
        program: &Program,
        resume_frame_index: usize,
        frame_state: FrameStateId,
    ) -> RuntimeResult<Continuation> {
        let mut continuation = Continuation::empty();

        // capture live frame bytes in call order
        for frame_index in 0..self.frames.len() {
            let frame = &self.frames[frame_index];
            let frame_state =
                self.frame_state(program, frame, frame_index, resume_frame_index, frame_state)?;
            let byte_offset = continuation.push_frame_bytes(frame.bytes());

            continuation.frames.push(ContinuationFrame {
                frame_state,
                normal_state: frame.invocation.map(|invocation| invocation.normal_state),
                unwind_state: frame.invocation.map(|invocation| invocation.unwind_state),
                byte_offset,
                stack_offset: frame.stack_offset,
                byte_len: frame.byte_len(),
            });
        }

        // clear active execution storage after capture
        self.frames.clear();
        self.stack.reset();

        Ok(continuation)
    }

    /// Restore one continuation into active VM frame storage.
    pub(crate) fn restore_continuation(
        &mut self,
        continuation: &Continuation,
        program: &Program,
    ) -> RuntimeResult<(Vec<Frame>, usize, FrameStateId)> {
        if continuation.frames.is_empty() {
            return Err(RuntimeError::new(Error::invalid_continuation()));
        }

        self.stack.reset();
        let mut frames = Vec::with_capacity(continuation.frames.len());

        // rebuild active frame storage from continuation bytes
        for frame in &continuation.frames {
            let bytes = continuation
                .frame_bytes(frame)
                .ok_or_else(|| RuntimeError::new(Error::invalid_continuation()))?;
            self.stack.restore_bytes(frame.stack_offset, bytes)?;

            let frame_base = self.stack.address(frame.stack_offset, bytes.len())?;
            let frame = Frame::from_continuation_frame(frame, program, frame_base)?;
            frames.push(frame);
        }

        let resume_frame_index = continuation.frames.len() - 1;
        let frame_state = continuation
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_continuation()))?
            .frame_state;

        Ok((frames, resume_frame_index, frame_state))
    }

    /// Visit mutable heap root slots referenced by one continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        let program = self.program.clone();

        Self::visit_continuation_slots(program.as_ref(), continuation, visit)
            .map_err(RuntimeError::new)
    }

    /// Visit mutable heap root slots from one continuation.
    pub fn visit_continuation_slots(
        program: &Program,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame_index in 0..continuation.frames.len() {
            Self::visit_continuation_frame_root_slots(frame_index, continuation, program, visit)?;
        }

        Ok(())
    }

    /// Return the frame state for one live frame.
    fn frame_state(
        &self,
        program: &Program,
        frame: &Frame,
        frame_index: usize,
        resume_frame_index: usize,
        resume_frame_state: FrameStateId,
    ) -> Result<FrameStateId, Error> {
        if frame_index == resume_frame_index {
            return Ok(resume_frame_state);
        }

        frame.state(program).map_err(|error| error.error)
    }

    /// Visit mutable heap root slots from one captured frame.
    fn visit_continuation_frame_root_slots(
        frame_index: usize,
        continuation: &mut Continuation,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let (layout, materialization) =
            Self::continuation_frame_materialization(frame_index, continuation, program)?;

        visit_materialized_slots(program, layout, materialization, |slot| {
            Self::visit_continuation_frame_slot_root_slots(
                frame_index,
                continuation,
                program,
                slot,
                visit,
            )
        })
    }

    /// Return the layout and materialization for one captured frame.
    fn continuation_frame_materialization<'a>(
        frame_index: usize,
        continuation: &Continuation,
        program: &'a Program,
    ) -> Result<(&'a FrameLayout, &'a FrameMaterialization), Error> {
        let frame = continuation
            .frames
            .get(frame_index)
            .ok_or(Error::invalid_continuation())?;
        let materialization = program
            .frame_materialization(frame.frame_state)
            .ok_or(Error::invalid_continuation())?;
        let layout = program
            .frame_layout_by_id(materialization.frame_layout)
            .ok_or(Error::invalid_continuation())?;
        if frame.byte_len() < layout.byte_len() as usize {
            return Err(Error::invalid_continuation());
        }

        Ok((layout, materialization))
    }

    /// Visit mutable heap root slots from one captured frame slot.
    fn visit_continuation_frame_slot_root_slots(
        frame_index: usize,
        continuation: &mut Continuation,
        program: &Program,
        slot: &FrameSlot,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let start = slot.offset as usize;
        let end = start + slot.byte_len() as usize;
        let frame_bytes = continuation
            .frame_bytes_mut(frame_index)
            .ok_or(Error::invalid_continuation())?;
        let bytes = frame_bytes
            .get_mut(start..end)
            .ok_or(Error::invalid_continuation())?;

        visit_frame_slot_root_slots(program, slot, bytes, visit)
    }
}
