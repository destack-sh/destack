use std::sync::Arc;

use destack_bytecode::{CodeOffset, FrameMap, RegisterSpan};
use destack_heap::{HeapResult, RootSlot};
use destack_memory::{MemoryImage, MemoryRange};
use destack_program::{
    FrameLayout, FramePoint, FrameSlot, FrameStateId, FunctionId, Program, ProgramPoint, TypeId,
    Word,
};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Error, Result};

use super::{Frame, FrameRestore, Machine};

/// Immutable image produced by one bytecode machine.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineImage {
    /// Physical frames in caller to callee order.
    frames: Arc<[FrameImage]>,
    /// Retained stack allocation inside world memory.
    stack: MemoryRange,
    /// Live byte prefix inside the retained stack allocation.
    byte_len: usize,
}

/// One physical frame inside a machine image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct FrameImage {
    /// Canonical frame state describing live values.
    state: FrameStateId,
    /// Physical VM frame descriptor.
    frame: Frame,
}

/// One physical frame and its canonical maps during capture or restore.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FrameMapping {
    /// Physical frame descriptor.
    frame: Frame,
    /// Canonical frame state identity.
    pub(crate) state: FrameStateId,
    /// Canonical frame layout.
    layout: FrameLayout,
    /// Physical bytecode register map.
    map: FrameMap,
    /// Canonical byte offset inside the complete image.
    byte_offset: usize,
}

impl FrameMapping {
    /// Return canonical slots paired with their physical register spans.
    fn values<'a>(&'a self, program: &'a Program) -> Result<(&'a [FrameSlot], &'a [RegisterSpan])> {
        let slots = program.frame_slots(&self.layout);
        let spans = self
            .map
            .registers(program.bytecode().registers(program.sections()));
        if slots.len() != spans.len() {
            return Err(Error::invalid_image());
        }

        Ok((slots, spans))
    }
}

impl MachineImage {
    /// Fork this immutable image through copy-on-write stack bytes.
    pub fn fork(&self) -> Self {
        Self {
            frames: self.frames.clone(),
            stack: self.stack,
            byte_len: self.byte_len,
        }
    }

    /// Return whether this image contains no active execution.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Return the captured physical frame count.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Return one captured physical frame state.
    pub fn frame_state(&self, index: usize) -> Option<FrameStateId> {
        self.frames.get(index).map(|frame| frame.state)
    }

    /// Project one captured physical frame into its canonical live value layout.
    pub fn frame_bytes(
        &self,
        memory: &MemoryImage,
        program: &Program,
        index: usize,
    ) -> Result<Vec<u8>> {
        let image = self.frames.get(index).ok_or_else(Error::invalid_image)?;
        let state = program
            .frame_state(image.state)
            .ok_or_else(Error::invalid_image)?;
        let layout = program
            .frame_layout(state.layout)
            .ok_or_else(Error::invalid_image)?;
        let map = program
            .bytecode()
            .frame(program.sections(), image.state.index())
            .ok_or_else(Error::invalid_image)?;
        let spans = map.registers(program.bytecode().registers(program.sections()));
        let slots = program.frame_slots(layout);
        if slots.len() != spans.len() {
            return Err(Error::invalid_image());
        }
        let mut bytes = vec![0; layout.byte_len() as usize];

        // project every live register span into its Program frame slot
        for (slot, span) in slots.iter().zip(spans) {
            if span.end() > image.frame.register_count as u32 {
                return Err(Error::invalid_image());
            }
            let source_offset = image.frame.range(*span) * Word::BYTE_LEN;
            let source = memory
                .read_bytes(self.stack.offset + source_offset, slot.byte_len as usize)
                .map_err(|_| Error::invalid_image())?;
            let target_offset = slot.offset as usize;
            let target_end = target_offset + slot.byte_len as usize;
            let target = bytes
                .get_mut(target_offset..target_end)
                .ok_or_else(Error::invalid_image)?;
            target.copy_from_slice(&source);
        }

        Ok(bytes)
    }
}

impl Machine {
    /// Return linked destructors for live values in one physical frame suffix.
    pub(crate) fn frame_destructors(
        &self,
        first_frame: usize,
        active_state: FrameStateId,
    ) -> Result<Vec<(FunctionId, usize)>> {
        let mappings = self.active_frame_mappings_from(first_frame, active_state)?;
        let mut destructors = Vec::new();

        // retain caller to callee and slot acquisition order
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program)?;

            for (slot, span) in slots.iter().zip(spans) {
                let Some(function) = self.program.destructor(slot.ty).map_err(Error::program)?
                else {
                    continue;
                };
                let byte_offset = mapping.frame.range(*span) * Word::BYTE_LEN;
                destructors.push((function, byte_offset));
            }
        }

        Ok(destructors)
    }

    /// Capture the retained physical call stack.
    pub fn capture(&self) -> Result<MachineImage> {
        if self.frames.is_empty() {
            return Ok(MachineImage::default());
        }
        let active_state = self.active_state()?;
        let mappings = self.active_frame_mappings_from(0, active_state)?;
        let frames = mappings
            .iter()
            .map(|mapping| FrameImage {
                state: mapping.state,
                frame: mapping.frame,
            })
            .collect::<Vec<_>>()
            .into();
        Ok(MachineImage {
            frames,
            stack: self.stack.range(),
            byte_len: self.stack.byte_len(),
        })
    }

    /// Restore one retained physical call stack.
    pub fn restore(&mut self, image: &MachineImage) -> Result<()> {
        if !self.frames.is_empty() {
            return Err(Error::execution_active());
        }

        let result = self.restore_machine(image);
        if result.is_err() {
            self.clear();
        }

        result
    }

    /// Restore one physical machine image after its Program was linked.
    fn restore_machine(&mut self, image: &MachineImage) -> Result<()> {
        if image.is_empty() {
            return Ok(());
        }

        self.stack.restore(image.stack, image.byte_len)?;
        self.frames = image.frames.iter().map(|image| image.frame).collect();

        Ok(())
    }

    /// Visit mutable heap roots retained by the physical call stack.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        if self.frames.is_empty() {
            return Ok(());
        }
        let active_state = self.active_state()?;
        let mappings = self.active_frame_mappings_from(0, active_state)?;
        let program = &self.program;
        let stack = &mut self.stack;

        // visit each live physical value through its canonical Program type
        for mapping in mappings {
            let (slots, spans) = mapping.values(program)?;
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

    /// Build canonical mappings for every retained physical frame.
    pub(crate) fn active_frame_mappings_from(
        &self,
        first_frame: usize,
        active_state: FrameStateId,
    ) -> Result<Vec<FrameMapping>> {
        let frames = &self.frames[first_frame..];
        let mut mappings = Vec::with_capacity(frames.len());
        let mut byte_offset = 0usize;

        // map callers at their call operations and the active frame at its instruction
        for (relative_index, frame) in frames.iter().copied().enumerate() {
            let index = first_frame + relative_index;
            let state = match self.frames.get(index + 1) {
                Some(callee) => match callee.return_to.state() {
                    Some(state) => state,
                    None => {
                        let pc = callee.return_to.pc().ok_or_else(Error::invalid_image)?;

                        self.frame_state_at(frame, pc)?
                    }
                },
                None => active_state,
            };
            let layout = self.frame_layout(state)?;
            byte_offset = byte_offset.next_multiple_of(layout.alignment() as usize);
            let mapping = self.frame_mapping(frame, state, byte_offset)?;
            byte_offset += layout.byte_len() as usize;
            mappings.push(mapping);
        }

        Ok(mappings)
    }

    /// Resolve the canonical state of the active physical frame.
    fn active_state(&self) -> Result<FrameStateId> {
        let frame = self
            .frames
            .last()
            .copied()
            .ok_or_else(Error::invalid_image)?;

        self.frame_state_at(frame, frame.pc)
    }

    /// Allocate physical frames described by one immutable image.
    pub(crate) fn restore_frame_mappings(
        &mut self,
        frames: &[FrameRestore],
    ) -> Result<Vec<FrameMapping>> {
        let mut mappings = Vec::with_capacity(frames.len());
        let mut byte_offset = 0usize;

        // allocate every frame before restoring cross-frame addresses
        for image in frames {
            let state = self
                .program
                .frame_state(image.state)
                .copied()
                .ok_or_else(Error::invalid_image)?;
            let linked = self
                .program
                .bytecode()
                .function(self.program.sections(), state.point.function().index())
                .ok_or_else(|| Error::undefined_function(state.point.function()))?;
            let code = linked
                .code()
                .ok_or_else(|| Error::undefined_function(state.point.function()))?;
            let layout = self.frame_layout(image.state)?;
            byte_offset = byte_offset.next_multiple_of(layout.alignment() as usize);
            if image.byte_offset != byte_offset {
                return Err(Error::invalid_image());
            }
            let register_offset = self.stack.push_words(linked.register_count())?;
            let mut frame = Frame::new(
                state.point.function(),
                code,
                register_offset,
                linked.register_count,
                image.return_to,
            );
            frame.pc = image.pc;
            self.frames.push(frame);
            mappings.push(self.frame_mapping(frame, image.state, byte_offset)?);
            byte_offset += layout.byte_len() as usize;
        }

        Ok(mappings)
    }

    /// Resolve one physical frame map and canonical layout.
    fn frame_mapping(
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
        let layout = self.frame_layout(state)?;
        let map = self.frame_map(state)?;

        Ok(FrameMapping {
            frame,
            state,
            layout,
            map,
            byte_offset,
        })
    }

    /// Resolve one frame state at a physical function operation.
    pub(crate) fn frame_state_at(&self, frame: Frame, pc: CodeOffset) -> Result<FrameStateId> {
        let bytecode = self.program.bytecode();
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

    /// Resolve one canonical frame layout.
    pub(crate) fn frame_layout(&self, state: FrameStateId) -> Result<FrameLayout> {
        let state = self
            .program
            .frame_state(state)
            .ok_or_else(Error::invalid_image)?;
        self.program
            .frame_layout(state.layout)
            .copied()
            .ok_or_else(Error::invalid_image)
    }

    /// Resolve one physical frame map.
    pub(crate) fn frame_map(&self, state: FrameStateId) -> Result<FrameMap> {
        self.program
            .bytecode()
            .frame(self.program.sections(), state.index())
            .copied()
            .ok_or_else(Error::invalid_image)
    }

    /// Pack live physical registers into canonical frame bytes.
    pub(crate) fn pack_frames(&self, mappings: &[FrameMapping]) -> Result<Vec<u8>> {
        let byte_len = mappings.last().map_or(0, |mapping| {
            mapping.byte_offset + mapping.layout.byte_len() as usize
        });
        let mut bytes = vec![0; byte_len];

        // copy every live value before rewriting embedded frame addresses
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program)?;
            for (slot, span) in slots.iter().zip(spans) {
                if span.end() > mapping.frame.register_count as u32 {
                    return Err(Error::invalid_image());
                }
                let target = Self::slot_bytes(
                    &mut bytes,
                    mapping.byte_offset + slot.offset as usize,
                    slot.byte_len as usize,
                )?;
                let source = mapping.frame.range(*span) * Word::BYTE_LEN;
                self.stack.read_bytes(source, target)?;
            }
        }

        // canonicalize every frame address against the complete physical stack
        for mapping in mappings {
            let slots = self.program.frame_slots(&mapping.layout);
            for slot in slots {
                let bytes = Self::slot_bytes(
                    &mut bytes,
                    mapping.byte_offset + slot.offset as usize,
                    slot.byte_len as usize,
                )?;
                self.pack_frame_addresses(slot.ty, bytes, mappings)?;
            }
        }

        Ok(bytes)
    }

    /// Restore canonical frame bytes into physical registers.
    pub(crate) fn unpack_frames(&mut self, mappings: &[FrameMapping], bytes: &[u8]) -> Result<()> {
        let expected = mappings.last().map_or(0, |mapping| {
            mapping.byte_offset + mapping.layout.byte_len() as usize
        });
        if bytes.len() != expected {
            return Err(Error::invalid_image());
        }
        let mut bytes = bytes.to_vec();

        // restore every frame address before copying values into registers
        for mapping in mappings {
            let slots = self.program.frame_slots(&mapping.layout);
            for slot in slots {
                let bytes = Self::slot_bytes(
                    &mut bytes,
                    mapping.byte_offset + slot.offset as usize,
                    slot.byte_len as usize,
                )?;
                self.restore_frame_addresses(slot.ty, bytes, mappings)?;
            }
        }

        // copy canonical live values into their physical register spans
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program)?;
            for (slot, span) in slots.iter().zip(spans) {
                let source = Self::slot_bytes(
                    &mut bytes,
                    mapping.byte_offset + slot.offset as usize,
                    slot.byte_len as usize,
                )?;
                let target = mapping.frame.range(*span) * Word::BYTE_LEN;
                self.stack.write_bytes(target, source)?;
            }
        }

        Ok(())
    }

    /// Encode physical frame addresses as canonical image offsets.
    fn pack_frame_addresses(
        &self,
        ty: TypeId,
        bytes: &mut [u8],
        mappings: &[FrameMapping],
    ) -> Result<()> {
        let mut is_valid = true;
        self.program
            .visit_byte_frame_addresses(ty, bytes, &mut |address| {
                let Some(word) = Word::from_bytes(address) else {
                    is_valid = false;

                    return Ok(());
                };
                if word.bits() == 0 {
                    return Ok(());
                }
                let Some(physical_offset) = (word.bits() as usize).checked_sub(1) else {
                    is_valid = false;

                    return Ok(());
                };
                let canonical_offset = self.canonical_offset(physical_offset, mappings);
                let Ok(Some(canonical_offset)) = canonical_offset else {
                    is_valid = false;

                    return Ok(());
                };
                address.copy_from_slice(&Word::from_bits((canonical_offset + 1) as u64).to_bytes());

                Ok(())
            })
            .map_err(|_| Error::invalid_image())?;
        if !is_valid {
            return Err(Error::invalid_image());
        }

        Ok(())
    }

    /// Decode canonical image offsets as physical frame addresses.
    fn restore_frame_addresses(
        &self,
        ty: TypeId,
        bytes: &mut [u8],
        mappings: &[FrameMapping],
    ) -> Result<()> {
        let mut is_valid = true;
        self.program
            .visit_byte_frame_addresses(ty, bytes, &mut |address| {
                let Some(word) = Word::from_bytes(address) else {
                    is_valid = false;

                    return Ok(());
                };
                if word.bits() == 0 {
                    return Ok(());
                }
                let Some(physical_offset) = (word.bits() as usize).checked_sub(1) else {
                    is_valid = false;

                    return Ok(());
                };
                let physical_offset = self.physical_offset(physical_offset, mappings);
                let Ok(Some(physical_offset)) = physical_offset else {
                    is_valid = false;

                    return Ok(());
                };
                address.copy_from_slice(&Word::from_bits((physical_offset + 1) as u64).to_bytes());

                Ok(())
            })
            .map_err(|_| Error::invalid_image())?;
        if !is_valid {
            return Err(Error::invalid_image());
        }

        Ok(())
    }

    /// Translate one physical frame offset into a canonical image offset.
    fn canonical_offset(&self, offset: usize, mappings: &[FrameMapping]) -> Result<Option<usize>> {
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program)?;
            let Some(physical) = offset.checked_sub(mapping.frame.byte_offset()) else {
                continue;
            };

            for (slot, span) in slots.iter().zip(spans) {
                let start = span.start.index() * Word::BYTE_LEN;
                let end = start + slot.byte_len as usize;
                if (start..end).contains(&physical) {
                    let canonical = mapping.byte_offset + slot.offset as usize + physical - start;

                    return Ok(Some(canonical));
                }
            }
        }

        Ok(None)
    }

    /// Translate one canonical image offset into a physical frame offset.
    fn physical_offset(&self, offset: usize, mappings: &[FrameMapping]) -> Result<Option<usize>> {
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program)?;

            for (slot, span) in slots.iter().zip(spans) {
                let start = mapping.byte_offset + slot.offset as usize;
                let end = start + slot.byte_len as usize;
                if (start..end).contains(&offset) {
                    let physical = span.start.index() * Word::BYTE_LEN + offset - start;

                    return Ok(Some(mapping.frame.byte_offset() + physical));
                }
            }
        }

        Ok(None)
    }

    /// Borrow one canonical slot from mutable image bytes.
    fn slot_bytes(bytes: &mut [u8], offset: usize, byte_len: usize) -> Result<&mut [u8]> {
        let end = offset + byte_len;

        bytes.get_mut(offset..end).ok_or_else(Error::invalid_image)
    }
}
