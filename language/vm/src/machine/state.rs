use tspp_bytecode::{Code, CodeOffset, FrameMap, RegisterSpan};
use tspp_heap::{HeapResult, RootSlot};
use tspp_program::{FrameLayout, FramePoint, FrameSlot, FrameStateId, Program, ProgramPoint, Word};

use crate::diagnostic::{Error, Result};

use super::{Fiber, Frame, Machine};

/// One physical frame and its canonical maps.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FrameMapping {
    /// Physical frame descriptor.
    pub(crate) frame: Frame,
    /// Canonical frame state identity.
    pub(crate) state: FrameStateId,
    /// Canonical frame layout.
    pub(crate) layout: FrameLayout,
    /// Physical bytecode register map.
    map: FrameMap,
    /// Canonical byte offset inside the packed activation image.
    pub(crate) byte_offset: usize,
}

impl FrameMapping {
    /// Return canonical slots paired with their physical register spans.
    pub(crate) fn values<'a>(
        &'a self,
        program: &'a Program,
        bytecode: Code,
    ) -> Result<(&'a [FrameSlot], &'a [RegisterSpan])> {
        let slots = program.frame_slots(&self.layout);
        let spans = self.map.registers(bytecode.registers(program.sections()));
        if slots.len() != spans.len() {
            return Err(Error::invalid_image());
        }

        Ok((slots, spans))
    }
}

impl Machine {
    /// Visit mutable heap roots retained by one fiber.
    pub fn visit_root_slots(
        &self,
        fiber: &mut Fiber,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        if fiber.frames.is_empty() {
            return Ok(());
        }
        let active_state = self.active_state(fiber)?;
        let mappings = self.frame_mappings(&fiber.frames, active_state)?;
        let program = &self.program;
        let stack = &mut fiber.stack;

        // visit each live physical value through its canonical Program type
        for mapping in mappings {
            let (slots, spans) = mapping.values(program, self.bytecode)?;
            for (slot, span) in slots.iter().zip(spans) {
                let byte_offset = mapping.frame.range(*span) * Word::BYTE_LEN;
                let bytes = stack.bytes_mut(byte_offset, slot.byte_len as usize)?;
                program
                    .visit_byte_root_slots(slot.ty, bytes, visit)
                    .map_err(Error::program)?;
            }
        }

        Ok(())
    }

    /// Build canonical mappings for one retained physical frame suffix.
    pub(crate) fn frame_mappings(
        &self,
        frames: &[Frame],
        active_state: FrameStateId,
    ) -> Result<Vec<FrameMapping>> {
        let mut mappings = Vec::with_capacity(frames.len());
        let mut byte_offset = 0usize;

        // map callers at their call operations and the active frame at its instruction
        for (index, frame) in frames.iter().copied().enumerate() {
            let state = match frames.get(index + 1) {
                Some(callee) => match callee.return_to.state() {
                    Some(state) => state,
                    None => {
                        let pc = callee.return_to.pc().ok_or_else(Error::invalid_image)?;

                        self.frame_state_at(frame, pc)?
                    }
                },
                None => active_state,
            };
            let mapping = self.frame_mapping(frame, state, byte_offset)?;
            byte_offset = mapping.byte_offset + mapping.layout.byte_len() as usize;
            mappings.push(mapping);
        }

        Ok(mappings)
    }

    /// Project one fiber image's frames into canonical states and packed bytes.
    pub fn project_frames(
        &self,
        image: &super::FiberImage,
        read: &mut dyn FnMut(usize, usize) -> Option<Vec<u8>>,
    ) -> Result<Vec<(FrameStateId, Vec<u8>)>> {
        let Some(active) = image.frames.last().copied() else {
            return Ok(Vec::new());
        };
        let active_state = self.frame_state_at(active, active.pc)?;
        let mappings = self.frame_mappings(&image.frames, active_state)?;
        let mut projected = Vec::with_capacity(mappings.len());

        // gather each frame's live registers into its canonical layout
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;
            let mut bytes = vec![0u8; mapping.layout.byte_len() as usize];
            for (slot, span) in slots.iter().zip(spans) {
                let stack_offset = mapping.frame.range(*span) * Word::BYTE_LEN;
                let offset = image.stack_range.offset + stack_offset;
                let value =
                    read(offset, slot.byte_len as usize).ok_or_else(Error::invalid_image)?;
                let start = slot.offset as usize;
                bytes[start..start + value.len()].copy_from_slice(&value);
            }
            projected.push((mapping.state, bytes));
        }

        Ok(projected)
    }

    /// Resolve the canonical state of the active physical frame.
    fn active_state(&self, fiber: &Fiber) -> Result<FrameStateId> {
        let frame = fiber
            .frames
            .last()
            .copied()
            .ok_or_else(Error::invalid_image)?;

        self.frame_state_at(frame, frame.pc)
    }

    /// Resolve one physical frame map and canonical layout at one packed offset.
    pub(crate) fn frame_mapping(
        &self,
        frame: Frame,
        state: FrameStateId,
        byte_offset: usize,
    ) -> Result<FrameMapping> {
        let linked = self
            .program
            .frame_state(state)
            .ok_or_else(Error::invalid_image)?;
        if linked.point.function() != frame.function {
            return Err(Error::invalid_image());
        }
        let layout = self
            .program
            .frame_layout(linked.layout)
            .copied()
            .ok_or_else(Error::invalid_image)?;
        let map = self.frame_map(state)?;

        Ok(FrameMapping {
            frame,
            state,
            byte_offset: byte_offset.next_multiple_of(layout.alignment() as usize),
            layout,
            map,
        })
    }

    /// Resolve one frame state at a physical function operation.
    pub(crate) fn frame_state_at(&self, frame: Frame, pc: CodeOffset) -> Result<FrameStateId> {
        let bytecode = self.bytecode;
        let function = bytecode
            .function(self.program.sections(), frame.function.index())
            .ok_or_else(|| Error::undefined_function(frame.function))?;
        let operation = function
            .operation_at(bytecode.operations(self.program.sections()), pc)
            .ok_or_else(Error::invalid_image)?;
        let point = ProgramPoint::new(frame.function, operation);

        self.program
            .frame_state_at(FramePoint::operation(point))
            .ok_or_else(Error::invalid_image)
    }

    /// Return the innermost retained program point of one fiber.
    pub fn fiber_point(&self, fiber: &Fiber) -> Option<ProgramPoint> {
        let frame = fiber.frames.last().copied()?;

        self.program_point_at(frame, frame.pc).ok()
    }

    /// Resolve one physical bytecode cursor into its exact Program point.
    fn program_point_at(&self, frame: Frame, pc: CodeOffset) -> Result<ProgramPoint> {
        let operation = self
            .bytecode
            .operation_at(self.program.sections(), frame.function.index(), pc)
            .ok_or_else(Error::invalid_image)?;

        Ok(ProgramPoint::new(frame.function, operation))
    }

    /// Resolve one physical frame map.
    pub(crate) fn frame_map(&self, state: FrameStateId) -> Result<FrameMap> {
        self.bytecode
            .frame(self.program.sections(), state.index())
            .copied()
            .ok_or_else(Error::invalid_image)
    }
}
