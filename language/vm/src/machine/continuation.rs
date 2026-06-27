use super::{
    Frame, FrameImage, Stack, StackImage, visit_frame_slot_root_slots, visit_materialized_slots,
};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::options::MachineOptions;
use destack_heap::{HeapResult, RootSlot};
use destack_program::{
    ContinuationImage, FrameLayout, FrameMaterialization, FrameSlot, FrameStateId, Program,
};

/// Suspended machine state captured at a yield terminator.
#[derive(Debug)]
pub struct Continuation {
    /// Page-backed stack bytes captured with this continuation.
    pub(crate) stack: Stack,
    /// The frame stack for the suspended execution.
    pub(crate) frames: Vec<Frame>,
    /// The frame index to resume execution in.
    pub(crate) resume_frame_index: usize,
    /// The frame state for this continuation.
    pub(crate) frame_state: FrameStateId,
}

impl Continuation {
    /// Fork this continuation for multi-shot resumption.
    pub fn fork(&self) -> RuntimeResult<Self> {
        let stack = self.stack.fork()?;
        let mut frames = Vec::with_capacity(self.frames.len());

        // clone frames over the forked stack bytes
        for frame in &self.frames {
            let base = stack
                .address(frame.stack_offset, frame.byte_len)
                .map_err(|_| RuntimeError::new(Error::invalid_continuation()))?;
            frames.push(frame.fork(base));
        }

        Ok(Self {
            stack,
            frames,
            resume_frame_index: self.resume_frame_index,
            frame_state: self.frame_state,
        })
    }

    /// Capture one immutable continuation image.
    pub fn image(&self, program: &Program) -> RuntimeResult<ContinuationImage> {
        let stack = self.stack.image()?;
        let frames = self
            .frames
            .iter()
            .enumerate()
            .map(|(frame_index, frame)| {
                let (frame_state, _materialization) = self
                    .frame_state_and_materialization(program, frame, frame_index)
                    .map_err(RuntimeError::new)?;

                Ok(capture_frame_image(frame, frame_state))
            })
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(ContinuationImage { stack, frames })
    }

    /// Visit mutable heap root slots referenced by this continuation.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame_index in 0..self.frames.len() {
            let materialization = {
                let frame = &self.frames[frame_index];
                self.frame_materialization(program, frame, frame_index)?
            };

            self.frames[frame_index].visit_materialized_root_slots(
                program,
                materialization,
                visit,
            )?;
        }

        Ok(())
    }

    /// Visit mutable heap root slots from one captured continuation image.
    pub(crate) fn visit_image_root_slots(
        image: &mut ContinuationImage,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame in &mut image.frames {
            visit_frame_image_root_slots(frame, &mut image.stack, program, visit)?;
        }

        Ok(())
    }

    /// Rebuild one continuation from an immutable image.
    pub(crate) fn from_image(
        image: &ContinuationImage,
        program: &Program,
        options: &MachineOptions,
    ) -> RuntimeResult<Self> {
        if image.frames.is_empty() {
            return Err(RuntimeError::new(Error::invalid_continuation()));
        }

        let stack = Stack::from_image(&image.stack, options.limits.stack_bytes)?;
        let mut frames = Vec::with_capacity(image.frames.len());

        for frame_image in &image.frames {
            let materialization = program
                .frame_materialization(frame_image.frame_state)
                .ok_or_else(|| RuntimeError::new(Error::invalid_continuation()))?;
            let layout = program
                .frame_layout_by_id(materialization.frame_layout)
                .ok_or_else(|| RuntimeError::new(Error::invalid_continuation()))?;
            if frame_image.byte_len < layout.byte_len as usize
                || image
                    .stack
                    .frame_bytes(frame_image.stack_offset, frame_image.byte_len)
                    .is_none()
            {
                return Err(RuntimeError::new(Error::invalid_continuation()));
            }

            let frame_base = stack.address(frame_image.stack_offset, frame_image.byte_len)?;
            let frame =
                Frame::from_image(frame_image, program, frame_image.stack_offset, frame_base)?;
            frames.push(frame);
        }

        let resume_frame_index = image.frames.len() - 1;
        let frame_state = image
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_continuation()))?
            .frame_state;

        Ok(Self {
            stack,
            frames,
            resume_frame_index,
            frame_state,
        })
    }

    /// Return the frame materialization for one live continuation frame.
    fn frame_materialization<'a>(
        &self,
        program: &'a Program,
        frame: &Frame,
        frame_index: usize,
    ) -> Result<&'a FrameMaterialization, Error> {
        let (_frame_state, materialization) =
            self.frame_state_and_materialization(program, frame, frame_index)?;

        Ok(materialization)
    }

    /// Return the frame state and frame materialization for one live continuation frame.
    fn frame_state_and_materialization<'a>(
        &self,
        program: &'a Program,
        frame: &Frame,
        frame_index: usize,
    ) -> Result<(FrameStateId, &'a FrameMaterialization), Error> {
        let frame_state = self.frame_state(program, frame, frame_index)?;

        let frame_materialization =
            program.frame_materialization(frame_state).ok_or_else(|| {
                Error::internal(format!(
                    "missing frame materialization for frame state: {frame_state:?}"
                ))
            })?;

        debug_assert_eq!(
            frame_materialization.frame_layout,
            frame.frame_layout(),
            "vm safepoint materialization should target the captured frame layout"
        );

        Ok((frame_state, frame_materialization))
    }

    /// Return the frame state captured for one live continuation frame.
    fn frame_state(
        &self,
        program: &Program,
        frame: &Frame,
        frame_index: usize,
    ) -> Result<FrameStateId, Error> {
        if frame_index == self.resume_frame_index {
            return Ok(self.frame_state);
        }

        let block = frame.block;
        let point = program.point(frame.function(), block, frame.pc as u32);

        program.frame_state_at(point).ok_or_else(|| {
            Error::internal(format!(
                "missing frame state for frame position: {:?} {:?} {}",
                frame.function(),
                block,
                frame.pc
            ))
        })
    }
}

/// Capture one frame image from one live frame.
fn capture_frame_image(frame: &Frame, frame_state: FrameStateId) -> FrameImage {
    FrameImage {
        frame_state,
        return_state: frame.return_state,
        stack_offset: frame.stack_offset,
        byte_len: frame.byte_len,
    }
}

/// Visit mutable heap root slots from one captured frame.
fn visit_frame_image_root_slots(
    frame: &mut FrameImage,
    stack: &mut StackImage,
    program: &Program,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> Result<(), Error> {
    let (layout, materialization) = frame_image_materialization(frame, program)?;

    visit_materialized_slots(layout, materialization, |slot| {
        visit_frame_image_slot_root_slots(frame, stack, program, slot, visit)
    })
}

/// Return the layout and materialization for one captured frame.
fn frame_image_materialization<'a>(
    frame: &FrameImage,
    program: &'a Program,
) -> Result<(&'a FrameLayout, &'a FrameMaterialization), Error> {
    let materialization = program
        .frame_materialization(frame.frame_state)
        .ok_or(Error::invalid_continuation())?;
    let layout = program
        .frame_layout_by_id(materialization.frame_layout)
        .ok_or(Error::invalid_continuation())?;
    if frame.byte_len < layout.byte_len as usize {
        return Err(Error::invalid_continuation());
    }

    Ok((layout, materialization))
}

/// Visit mutable heap root slots from one captured frame slot.
fn visit_frame_image_slot_root_slots(
    frame: &mut FrameImage,
    stack: &mut StackImage,
    program: &Program,
    slot: &FrameSlot,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> Result<(), Error> {
    let start = slot.offset as usize;
    let end = start + slot.byte_len as usize;
    let frame_bytes = stack
        .frame_bytes_mut(frame.stack_offset, frame.byte_len)
        .ok_or(Error::invalid_continuation())?;
    let bytes = frame_bytes
        .get_mut(start..end)
        .ok_or(Error::invalid_continuation())?;

    visit_frame_slot_root_slots(program, slot, bytes, visit)
}
