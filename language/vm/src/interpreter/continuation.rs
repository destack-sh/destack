use destack_engine as engine;
use serde::{Deserialize, Serialize};

use super::{ExceptionalCall, Frame, Stack, visit_materialized_slots};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::options::IsolateOptions;
use crate::program::Program;
use crate::{RootSink, Word};
use destack_heap::{HeapResult, RootSlot};

/// Suspended interpreter state captured at a yield terminator.
#[derive(Debug)]
pub struct Continuation {
    /// The engine id used to validate the continuation.
    pub(crate) isolate_id: engine::EngineId,
    /// Page-backed stack bytes captured with this continuation.
    pub(crate) stack: Stack,
    /// The frame stack for the suspended execution.
    pub(crate) frames: Vec<Frame>,
    /// The frame index to resume execution in.
    pub(crate) resume_frame_index: usize,
    /// The frame state for this continuation.
    pub(crate) frame_state: engine::FrameStateId,
}

/// Immutable continuation image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationImage {
    /// The engine identity used to validate resumption.
    pub engine_id: engine::EngineId,
    /// The captured frames from outermost to innermost.
    pub frames: Vec<ContinuationFrame>,
}

/// Immutable frame image captured inside one continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationFrame {
    /// The frame layout used by this frame.
    pub frame_layout: engine::FrameLayoutId,
    /// The logical frame state captured by this frame.
    pub frame_state: engine::FrameStateId,
    /// The active exceptional call owned by this frame when another frame is active.
    pub exceptional_call: Option<ExceptionalCall>,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}

impl Continuation {
    /// Clone this continuation for multi-shot resumption.
    pub fn clone_for_fork(&self) -> RuntimeResult<Self> {
        let stack = self.stack.fork()?;
        let mut frames = Vec::with_capacity(self.frames.len());

        // clone frames over the forked stack bytes
        for frame in &self.frames {
            let base = stack
                .address(frame.stack_offset, frame.byte_len)
                .map_err(|_| RuntimeError::new(Error::InvalidContinuation))?;
            frames.push(frame.clone_for_fork(base));
        }

        Ok(Self {
            isolate_id: self.isolate_id,
            stack,
            frames,
            resume_frame_index: self.resume_frame_index,
            frame_state: self.frame_state,
        })
    }

    /// Capture one immutable continuation image.
    pub fn image(&self, program: &Program) -> RuntimeResult<ContinuationImage> {
        let frames = self
            .frames
            .iter()
            .enumerate()
            .map(|(frame_index, frame)| {
                let (frame_state, _materialization) = self
                    .frame_state_and_materialization(program, frame, frame_index)
                    .map_err(RuntimeError::new)?;

                Ok(ContinuationFrame::capture(frame, frame_state))
            })
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(ContinuationImage {
            engine_id: self.isolate_id,
            frames,
        })
    }

    /// Visit heap roots referenced by this continuation.
    pub(crate) fn visit_roots(
        &self,
        program: &Program,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        for (frame_index, frame) in self.frames.iter().enumerate() {
            let materialization = self.frame_materialization(program, frame, frame_index)?;

            frame.visit_materialized_roots(program, materialization, roots)?;
        }

        Ok(())
    }

    /// Visit mutable local root slots referenced by this continuation.
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

    /// Visit heap roots from one captured continuation image.
    pub(crate) fn visit_image_roots(
        image: &ContinuationImage,
        program: &Program,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        for frame in &image.frames {
            frame.visit_roots(program, roots)?;
        }

        Ok(())
    }

    /// Visit mutable local root slots from one captured continuation image.
    pub(crate) fn visit_image_root_slots(
        image: &mut ContinuationImage,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame in &mut image.frames {
            frame.visit_root_slots(program, visit)?;
        }

        Ok(())
    }

    /// Rebuild one continuation from an immutable image.
    pub(crate) fn from_image(
        image: &ContinuationImage,
        program: &Program,
        options: &IsolateOptions,
    ) -> RuntimeResult<Self> {
        if image.frames.is_empty() {
            return Err(RuntimeError::new(Error::InvalidContinuation));
        }

        let mut stack = Stack::reserve(options.limits.max_stack_bytes)?;
        let mut frames = Vec::with_capacity(image.frames.len());

        for frame_image in &image.frames {
            let layout = program
                .frame_layout_by_id(frame_image.frame_layout)
                .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
            if frame_image.bytes.len() != layout.byte_len as usize {
                return Err(RuntimeError::new(Error::InvalidContinuation));
            }

            let base = stack.allocate(frame_image.bytes.len(), Word::BYTE_LEN)?;
            stack.write(base, &frame_image.bytes)?;
            let frame_base = stack.address(base, frame_image.bytes.len())?;
            let frame = frame_image.restore(program, base, frame_base)?;
            frames.push(frame);
        }

        let resume_frame_index = image.frames.len() - 1;
        let frame_state = image
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?
            .frame_state;

        Ok(Self {
            isolate_id: image.engine_id,
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
    ) -> Result<&'a engine::FrameMaterialization, Error> {
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
    ) -> Result<(engine::FrameStateId, &'a engine::FrameMaterialization), Error> {
        let frame_state = self.frame_state(program, frame, frame_index)?;

        let frame_materialization =
            program.frame_materialization(frame_state).ok_or_else(|| {
                Error::InvariantViolation {
                    context: format!(
                        "missing frame materialization for frame state: {frame_state:?}"
                    ),
                }
            })?;

        debug_assert_eq!(
            frame_materialization.frame_state, frame_state,
            "vm safepoint materialization should target the captured frame state"
        );
        debug_assert_eq!(
            frame_materialization.frame_layout, frame.frame_layout,
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
    ) -> Result<engine::FrameStateId, Error> {
        if frame_index == self.resume_frame_index {
            return Ok(self.frame_state);
        }

        let point = program.point(frame.function(), frame.current_block(), frame.pc as u32);

        program
            .frame_state_at(point)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing frame state for frame position: {:?} {:?} {}",
                    frame.function(),
                    frame.current_block(),
                    frame.pc
                ),
            })
    }
}

impl ContinuationFrame {
    /// Capture one durable frame from one live frame.
    fn capture(frame: &Frame, frame_state: engine::FrameStateId) -> Self {
        Self {
            frame_layout: frame.frame_layout,
            frame_state,
            exceptional_call: frame.exceptional_call.clone(),
            bytes: frame.bytes().to_vec(),
        }
    }

    /// Visit heap roots from this captured frame.
    fn visit_roots(&self, program: &Program, roots: &mut impl RootSink) -> Result<(), Error> {
        let (layout, materialization) = self.materialization(program)?;

        visit_materialized_slots(layout, materialization, |slot| {
            self.visit_slot_roots(program, slot, roots)
        })
    }

    /// Visit mutable local root slots from this captured frame.
    fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let (layout, materialization) = self.materialization(program)?;

        visit_materialized_slots(layout, materialization, |slot| {
            self.visit_slot_root_slots(program, slot, visit)
        })
    }

    /// Restore one live frame from this captured frame.
    fn restore(
        &self,
        program: &Program,
        stack_offset: usize,
        frame_base: *mut u8,
    ) -> RuntimeResult<Frame> {
        let layout = program
            .frame_layout_by_id(self.frame_layout)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
        let point = program
            .point_for_frame_state(self.frame_state)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;

        let function_id = point.function;
        let block_id = point.block;
        let function_ptr = program.functions.pointer_for(function_id).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: function_id,
            })
        })?;
        let function = unsafe { function_ptr.as_ref() };

        let block_index = function
            .blocks
            .iter()
            .position(|block| block.mir_block == block_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidContinuation))?;
        let mut frame = Frame::new(
            self.frame_layout,
            function_ptr,
            block_index as u32,
            layout,
            stack_offset,
            frame_base,
        );
        frame.pc = point.instruction_index as usize;
        frame.exceptional_call = self.exceptional_call.clone();

        Ok(frame)
    }

    /// Resolve the layout and frame materialization for this captured frame.
    fn materialization<'a>(
        &self,
        program: &'a Program,
    ) -> Result<(&'a engine::FrameLayout, &'a engine::FrameMaterialization), Error> {
        let layout = program
            .frame_layout_by_id(self.frame_layout)
            .ok_or(Error::InvalidContinuation)?;
        if self.bytes.len() != layout.byte_len as usize {
            return Err(Error::InvalidContinuation);
        }

        let materialization = program
            .frame_materialization(self.frame_state)
            .ok_or(Error::InvalidContinuation)?;

        Ok((layout, materialization))
    }

    /// Visit heap roots from one frame slot.
    fn visit_slot_roots(
        &self,
        program: &Program,
        slot: &engine::FrameSlot,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let layout =
            program
                .layout_for_layout(slot.layout)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!(
                        "missing image frame layout for root scan: layout={:?}",
                        slot.layout
                    ),
                })?;
        let start = slot.offset as usize;
        let end = start + slot.byte_len as usize;
        let bytes = &self.bytes[start..end];

        if !layout.is_word() {
            return program.visit_byte_roots(program.type_for_layout(slot.layout), bytes, roots);
        }

        let word = Self::read_word(bytes)?;

        program.visit_value_root(program.type_for_layout(slot.layout), word, roots)
    }

    /// Visit mutable local root slots from one frame slot.
    fn visit_slot_root_slots(
        &mut self,
        program: &Program,
        slot: &engine::FrameSlot,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let start = slot.offset as usize;
        let end = start + slot.byte_len as usize;
        let bytes = &mut self.bytes[start..end];

        program.visit_byte_root_slots(program.type_for_layout(slot.layout), bytes, visit)
    }

    /// Read one word from captured frame bytes.
    fn read_word(bytes: &[u8]) -> Result<Word, Error> {
        if bytes.len() < Word::BYTE_LEN {
            return Err(Error::InvalidContinuation);
        }

        let mut raw = [0u8; Word::BYTE_LEN];
        raw.copy_from_slice(&bytes[..Word::BYTE_LEN]);

        Ok(Word::from_bits(u64::from_le_bytes(raw)))
    }
}
