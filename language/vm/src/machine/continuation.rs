use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::options::MachineOptions;
use destack_heap::{HeapResult, RootSlot};
use destack_mir::TraceMap;
use destack_program::vm::{Cell, FramePointer};
use destack_program::{
    Continuation, ContinuationFrame, FrameLayout, FrameMaterialization, FrameSlot, FrameStateId,
    Program,
};

use super::{Frame, Machine, Stack, visit_frame_slot_root_slots, visit_materialized_slots};

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
                return_state: frame.return_state,
                byte_offset,
                byte_len: frame.byte_len(),
            });
        }

        // encode frame pointers relative to the continuation byte store
        Self::encode_frame_pointers(program, &mut continuation, &self.frames)?;

        // clear active execution storage after capture
        self.frames.clear();
        self.stack.reset(self.options.limits.stack_bytes)?;

        Ok(continuation)
    }

    /// Restore one continuation into active VM frame storage.
    pub(crate) fn restore_continuation(
        continuation: &Continuation,
        program: &Program,
        options: &MachineOptions,
    ) -> RuntimeResult<(Stack, Vec<Frame>, usize, FrameStateId)> {
        if continuation.frames.is_empty() {
            return Err(RuntimeError::new(Error::invalid_continuation()));
        }

        let mut stack = Stack::new(options.limits.stack_bytes)?;
        let mut frames = Vec::with_capacity(continuation.frames.len());

        // rebuild active frame storage from continuation bytes
        for frame in &continuation.frames {
            let bytes = continuation
                .frame_bytes(frame)
                .ok_or_else(|| RuntimeError::new(Error::invalid_continuation()))?;
            let stack_offset = stack.allocate_uninit(bytes.len(), Cell::BYTE_LEN)?;
            stack.copy_bytes(stack_offset, bytes)?;

            let frame_base = stack.address(stack_offset, bytes.len())?;
            let frame = Frame::from_continuation_frame(frame, program, stack_offset, frame_base)?;
            frames.push(frame);
        }

        // decode frame pointers against the new active frame addresses
        Self::decode_frame_pointers(program, continuation, &mut frames)?;

        let resume_frame_index = continuation.frames.len() - 1;
        let frame_state = continuation
            .frames
            .last()
            .ok_or_else(|| RuntimeError::new(Error::invalid_continuation()))?
            .frame_state;

        Ok((stack, frames, resume_frame_index, frame_state))
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

    /// Encode frame pointers inside one captured continuation.
    fn encode_frame_pointers(
        program: &Program,
        continuation: &mut Continuation,
        frames: &[Frame],
    ) -> Result<(), Error> {
        let pointers = FramePointerMap::from_active_frames(frames, continuation);

        // rewrite each materialized frame pointer cell to continuation-relative form
        for frame_index in 0..continuation.frames.len() {
            let frame_state = continuation.frames[frame_index].frame_state;
            let frame_bytes = continuation
                .frame_bytes_mut(frame_index)
                .ok_or(Error::invalid_continuation())?;

            Self::rewrite_materialized_frame_pointers(
                program,
                frame_state,
                frame_bytes,
                |address| pointers.encode(address),
            )?;
        }

        Ok(())
    }

    /// Decode frame pointers inside active frame bytes after restore.
    fn decode_frame_pointers(
        program: &Program,
        continuation: &Continuation,
        frames: &mut [Frame],
    ) -> Result<(), Error> {
        let pointers = FramePointerMap::from_restored_frames(continuation, frames);

        // rewrite each materialized frame pointer cell to active frame addresses
        for (frame_index, frame) in frames.iter_mut().enumerate() {
            let frame_state = continuation.frames[frame_index].frame_state;
            let frame_bytes = frame.bytes_mut();

            Self::rewrite_materialized_frame_pointers(
                program,
                frame_state,
                frame_bytes,
                |offset| pointers.decode(offset),
            )?;
        }

        Ok(())
    }

    /// Rewrite all frame-pointer cells in one materialized frame.
    fn rewrite_materialized_frame_pointers(
        program: &Program,
        frame_state: FrameStateId,
        frame_bytes: &mut [u8],
        rewrite: impl FnMut(usize) -> Result<usize, Error>,
    ) -> Result<(), Error> {
        let materialization = program
            .frame_materialization(frame_state)
            .ok_or(Error::invalid_continuation())?;
        let layout = program
            .frame_layout_by_id(materialization.frame_layout)
            .ok_or(Error::invalid_continuation())?;

        // rewrite each copied slot because non-copied slots are dead at the safepoint
        let mut rewriter = FramePointerRewriter {
            bytes: frame_bytes,
            rewrite,
        };
        for slot in program.frame_copied_slots(materialization) {
            let slot = program
                .frame_slot(layout, *slot)
                .ok_or(Error::invalid_continuation())?;
            let layout = program
                .layout(slot.ty)
                .ok_or(Error::invalid_continuation())?;
            let trace_map = program.trace_map(layout.trace)?;

            rewriter.rewrite_trace_map(&trace_map, slot.offset as usize)?;
        }

        Ok(())
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

/// Rewrites frame pointers inside materialized continuation bytes.
struct FramePointerRewriter<'a, F>
where
    F: FnMut(usize) -> Result<usize, Error>,
{
    /// The frame bytes being rewritten.
    bytes: &'a mut [u8],
    /// The address rewrite operation.
    rewrite: F,
}

impl<F> FramePointerRewriter<'_, F>
where
    F: FnMut(usize) -> Result<usize, Error>,
{
    /// Rewrite all frame reference fields described by one trace map.
    fn rewrite_trace_map(&mut self, trace_map: &TraceMap, base_offset: usize) -> Result<(), Error> {
        match trace_map {
            TraceMap::Empty => {}
            TraceMap::Fixed { frame_offsets, .. } => {
                self.rewrite_fixed(frame_offsets, base_offset)?;
            }
            TraceMap::Nested { byte_offset, map } => {
                let base_offset = base_offset + *byte_offset as usize;

                self.rewrite_trace_map(map, base_offset)?;
            }
            TraceMap::Composite { maps } => {
                for map in maps {
                    self.rewrite_trace_map(map, base_offset)?;
                }
            }
            TraceMap::Repeated {
                count,
                stride,
                element,
            } => {
                for index in 0..*count {
                    let base_offset = base_offset + index as usize * *stride as usize;

                    self.rewrite_trace_map(element, base_offset)?;
                }
            }
            TraceMap::Tagged {
                tag_bytes,
                variants,
            } => {
                let tag = self.read_tag(base_offset, *tag_bytes)?;
                let variant = variants
                    .iter()
                    .find(|variant| variant.tag == tag)
                    .ok_or(Error::invalid_continuation())?;
                let base_offset = base_offset + variant.payload_offset as usize;

                self.rewrite_trace_map(&variant.map, base_offset)?;
            }
        }

        Ok(())
    }

    /// Rewrite fixed frame references at the given base offset.
    fn rewrite_fixed(&mut self, frame_offsets: &[u32], base_offset: usize) -> Result<(), Error> {
        for offset in frame_offsets {
            let byte_offset = base_offset + *offset as usize;

            self.rewrite_cell(byte_offset)?;
        }

        Ok(())
    }

    /// Read one active variant tag.
    fn read_tag(&self, byte_offset: usize, tag_bytes: u8) -> Result<u64, Error> {
        let end = byte_offset + tag_bytes as usize;
        let tag = self
            .bytes
            .get(byte_offset..end)
            .ok_or(Error::invalid_continuation())?;
        let mut raw = [0u8; std::mem::size_of::<u64>()];
        raw[..tag_bytes as usize].copy_from_slice(tag);

        Ok(u64::from_le_bytes(raw))
    }

    /// Rewrite one frame pointer cell in a byte range.
    fn rewrite_cell(&mut self, byte_offset: usize) -> Result<(), Error> {
        let end = byte_offset + Cell::BYTE_LEN;
        let cell = self
            .bytes
            .get_mut(byte_offset..end)
            .ok_or(Error::invalid_continuation())?;
        let value = Cell::from_byte_slice(cell).ok_or(Error::invalid_continuation())?;
        let value = (self.rewrite)(value.as_frame_pointer().address())?;
        let value = Cell::frame_pointer(FramePointer::from_address(value));

        cell.copy_from_slice(&value.to_byte_array());

        Ok(())
    }
}

/// Frame pointer mapping between active stack bytes and continuation bytes.
struct FramePointerMap {
    /// The mapped frame byte ranges.
    ranges: Vec<FramePointerRange>,
}

impl FramePointerMap {
    /// Build frame pointer mapping from active frames.
    fn from_active_frames(frames: &[Frame], continuation: &Continuation) -> Self {
        let ranges = frames
            .iter()
            .zip(continuation.frames.iter())
            .map(|(frame, continuation)| FramePointerRange {
                base: frame.base_address(),
                byte_len: frame.byte_len(),
                continuation_offset: continuation.byte_offset,
            })
            .collect();

        Self { ranges }
    }

    /// Build frame pointer mapping from restored frames.
    fn from_restored_frames(continuation: &Continuation, frames: &[Frame]) -> Self {
        let ranges = continuation
            .frames
            .iter()
            .zip(frames.iter())
            .map(|(continuation, frame)| FramePointerRange {
                base: frame.base_address(),
                byte_len: frame.byte_len(),
                continuation_offset: continuation.byte_offset,
            })
            .collect();

        Self { ranges }
    }

    /// Encode one active frame pointer as a continuation-relative offset.
    fn encode(&self, address: usize) -> Result<usize, Error> {
        if address == 0 {
            return Ok(0);
        }

        let range = self
            .ranges
            .iter()
            .find(|range| range.contains_address(address))
            .ok_or(Error::invalid_continuation())?;
        let offset = range.continuation_offset + address - range.base;

        // reserve zero for null frame pointers
        Ok(offset + 1)
    }

    /// Decode one continuation-relative frame pointer as an active frame address.
    fn decode(&self, offset: usize) -> Result<usize, Error> {
        if offset == 0 {
            return Ok(0);
        }

        // undo the null pointer bias
        let offset = offset - 1;
        let range = self
            .ranges
            .iter()
            .find(|range| range.contains_offset(offset))
            .ok_or(Error::invalid_continuation())?;
        let address = range.base + offset - range.continuation_offset;

        Ok(address)
    }
}

/// One mapped frame byte range.
struct FramePointerRange {
    /// Active frame byte address.
    base: usize,
    /// The frame byte width.
    byte_len: usize,
    /// Byte offset inside the continuation byte store.
    continuation_offset: usize,
}

impl FramePointerRange {
    /// Return whether this range contains one native frame address.
    fn contains_address(&self, address: usize) -> bool {
        let end = self.base + self.byte_len;

        self.base <= address && address < end
    }

    /// Return whether this range contains one continuation byte offset.
    fn contains_offset(&self, offset: usize) -> bool {
        let end = self.continuation_offset + self.byte_len;

        self.continuation_offset <= offset && offset < end
    }
}
