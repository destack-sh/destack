use destack_bytecode::{Code, CodeOffset, FrameMap, RegisterSpan};
use destack_heap::{HeapResult, RootSlot};
use destack_memory::MemoryRange;
use destack_mir as mir;
use destack_program::{
    ActivationImage, FrameImage, FrameLayout, FrameLink, FramePoint, FrameSlot, FrameStateId,
    FunctionId, Program, ProgramPoint, TypeId, Word,
};

use crate::diagnostic::{Error, Result};

use super::{Frame, Machine, Return};

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
    fn values<'a>(
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
    /// Return linked destructors for live values in one physical frame suffix.
    pub(crate) fn frame_destructors(
        &self,
        first_frame: usize,
        active_state: FrameStateId,
    ) -> Result<Vec<(FunctionId, usize)>> {
        let mappings = self.frame_mappings(first_frame, active_state)?;
        let mut destructors = Vec::new();

        // retain caller to callee and slot acquisition order
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;

            for (slot, span) in slots.iter().zip(spans) {
                let Some(function) = self
                    .program
                    .destructor(slot.ty, mir::Storage::Frame)
                    .map_err(Error::program)?
                else {
                    continue;
                };
                let byte_offset = mapping.frame.range(*span) * Word::BYTE_LEN;
                destructors.push((function, byte_offset));
            }
        }

        Ok(destructors)
    }

    /// Capture physical execution as the canonical activation.
    pub(crate) fn capture(&mut self, active_state: FrameStateId) -> Result<()> {
        if self.activation.is_some() {
            return Err(Error::execution_active());
        }
        let Some(Return::Exit { completion }) = self.frames.first().map(|frame| frame.return_to)
        else {
            return Err(Error::invalid_image());
        };
        let mappings = self.frame_mappings(0, active_state)?;
        let frames = mappings
            .iter()
            .copied()
            .map(|mapping| self.frame_image(mapping))
            .collect::<Result<Vec<_>>>()?;
        let bytes = self.pack_frames(&mappings)?;
        let memory = self.store_image(&bytes)?;
        self.activation = Some(ActivationImage::new(completion, frames, memory));
        self.clear_physical();

        Ok(())
    }

    /// Restore one canonical activation for later execution.
    pub fn restore(&mut self, image: ActivationImage) -> Result<()> {
        if !self.frames.is_empty() || self.activation.is_some() {
            return Err(Error::execution_active());
        }
        self.activation = Some(image);

        Ok(())
    }

    /// Materialize the captured activation into VM frames.
    pub(crate) fn materialize(&mut self) -> Result<()> {
        if !self.frames.is_empty() {
            return Err(Error::execution_active());
        }
        let image = self
            .activation
            .take()
            .ok_or_else(Error::execution_not_stopped)?;
        let result = self.materialize_image(&image);
        let release = image.release(&self.stack.memory()).map_err(Error::program);
        if result.is_err() || release.is_err() {
            self.clear_physical();
        }
        release?;

        result
    }

    /// Materialize one canonical activation into physical VM storage.
    fn materialize_image(&mut self, image: &ActivationImage) -> Result<()> {
        if image.frames().first().copied().map(FrameImage::link) != Some(FrameLink::Root) {
            return Err(Error::invalid_image());
        }
        let root_return = Return::Exit {
            completion: image.completion(),
        };
        let mappings = self.materialize_frames(image.frames(), root_return)?;
        let mut bytes = self.load_image(image.memory())?;
        self.unpack_frames(&mappings, &mut bytes)?;

        Ok(())
    }

    /// Visit mutable heap roots retained by this machine.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        if let Some(image) = &mut self.activation {
            return self
                .program
                .visit_activation_root_slots(&self.stack.memory(), image, visit)
                .map_err(Error::program);
        }
        if self.frames.is_empty() {
            return Ok(());
        }
        let active_state = self.active_state()?;

        self.visit_root_slots_at(active_state, visit)
    }

    /// Visit mutable heap roots through one exact active frame state.
    pub(crate) fn visit_root_slots_at(
        &mut self,
        active_state: FrameStateId,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let mappings = self.frame_mappings(0, active_state)?;
        let program = &self.program;
        let stack = &mut self.stack;

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

    /// Store canonical frame bytes inside world memory.
    pub(crate) fn store_image(&self, bytes: &[u8]) -> Result<MemoryRange> {
        self.stack
            .memory()
            .allocate_bytes(bytes, align_of::<Word>())
            .map_err(|error| Error::program(error.into()))
    }

    /// Load canonical frame bytes from world memory.
    pub(crate) fn load_image(&self, range: MemoryRange) -> Result<Vec<u8>> {
        self.stack
            .memory()
            .read_bytes(range.offset, range.byte_len)
            .map_err(|error| Error::program(error.into()))
    }

    /// Build canonical mappings for every retained physical frame.
    pub(crate) fn frame_mappings(
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

    /// Resolve one physical frame map and canonical layout.
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

    /// Convert one physical mapping into its canonical frame row.
    pub(crate) fn frame_image(&self, mapping: FrameMapping) -> Result<FrameImage> {
        let point = self.program_point_at(mapping.frame, mapping.frame.pc)?;

        Ok(FrameImage::new(
            mapping.state,
            point,
            mapping.frame.return_to.link(),
        ))
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

    /// Resolve one physical bytecode cursor into its exact Program point.
    fn program_point_at(&self, frame: Frame, pc: CodeOffset) -> Result<ProgramPoint> {
        let operation = self
            .bytecode
            .operation_at(self.program.sections(), frame.function.index(), pc)
            .ok_or_else(Error::invalid_image)?;

        Ok(ProgramPoint::new(frame.function, operation))
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
        self.bytecode
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
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;
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
    pub(crate) fn unpack_frames(
        &mut self,
        mappings: &[FrameMapping],
        bytes: &mut [u8],
    ) -> Result<()> {
        let expected = mappings.last().map_or(0, |mapping| {
            mapping.byte_offset + mapping.layout.byte_len() as usize
        });
        if bytes.len() != expected {
            return Err(Error::invalid_image());
        }
        // restore every frame address before copying values into registers
        for mapping in mappings {
            let slots = self.program.frame_slots(&mapping.layout);
            for slot in slots {
                let bytes = Self::slot_bytes(
                    bytes,
                    mapping.byte_offset + slot.offset as usize,
                    slot.byte_len as usize,
                )?;
                self.restore_frame_addresses(slot.ty, bytes, mappings)?;
            }
        }

        // copy canonical live values into their physical register spans
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;
            for (slot, span) in slots.iter().zip(spans) {
                let source = Self::slot_bytes(
                    bytes,
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
                if word.is_nullish() {
                    return Ok(());
                }
                let Some(physical_offset) = (word.bits() as usize).checked_sub(2) else {
                    is_valid = false;

                    return Ok(());
                };
                let canonical_offset = self.canonical_offset(physical_offset, mappings);
                let Ok(Some(canonical_offset)) = canonical_offset else {
                    is_valid = false;

                    return Ok(());
                };
                address.copy_from_slice(&Word::from_bits((canonical_offset + 2) as u64).to_bytes());

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
                if word.is_nullish() {
                    return Ok(());
                }
                let Some(physical_offset) = (word.bits() as usize).checked_sub(2) else {
                    is_valid = false;

                    return Ok(());
                };
                let physical_offset = self.physical_offset(physical_offset, mappings);
                let Ok(Some(physical_offset)) = physical_offset else {
                    is_valid = false;

                    return Ok(());
                };
                address.copy_from_slice(&Word::from_bits((physical_offset + 2) as u64).to_bytes());

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
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;
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
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;

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
