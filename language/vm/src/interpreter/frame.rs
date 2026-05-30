use std::mem;

use destack_engine as engine;
use destack_mir as mir;
use serde::{Deserialize, Serialize};

use crate::Word;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::program::{Function, Program, ProgramPoint};

/// Call frame in the interpreter.
///
/// The byte address is owned by the page-backed VM stack.
#[derive(Debug)]
pub struct Frame {
    /// Current MIR function id.
    pub(crate) function: mir::LocalNodeId<mir::Function>,
    /// The logical frame layout id.
    pub(crate) frame_layout: engine::FrameLayoutId,
    /// The current block index in the lowered function.
    pub(crate) block: u32,
    /// Program counter within the current block.
    pub(crate) pc: usize,
    /// The caller frame state after one callee returns.
    pub(crate) return_state: Option<engine::FrameStateId>,
    /// The byte offset in the interpreter stack arena.
    pub(crate) stack_offset: usize,
    /// The frame byte width.
    pub(crate) byte_len: usize,
    /// Native address of the frame bytes in the interpreter stack arena.
    base: usize,
}

/// Immutable frame image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameImage {
    /// The captured logical frame state.
    pub frame_state: engine::FrameStateId,
    /// The caller frame state after one callee returns.
    pub return_state: Option<engine::FrameStateId>,
    /// The byte offset inside the captured stack image.
    pub stack_offset: usize,
    /// The captured frame byte width.
    pub byte_len: usize,
}

// frame should fit in 64 bytes
const _: () = assert!(std::mem::size_of::<Frame>() <= 64);

impl Frame {
    /// Create a new frame for a function.
    pub(crate) fn new(
        function: &Function,
        block: u32,
        layout: &engine::FrameLayout,
        stack_offset: usize,
        base: usize,
    ) -> Self {
        Self {
            function: function.mir_function,
            frame_layout: function.frame_layout,
            block,
            pc: 0,
            return_state: None,
            stack_offset,
            byte_len: layout.byte_len as usize,
            base,
        }
    }

    /// Return the current MIR function id.
    #[inline(always)]
    pub(crate) fn function(&self) -> mir::LocalNodeId<mir::Function> {
        self.function
    }

    /// Return the logical frame layout id.
    #[inline(always)]
    pub(crate) fn frame_layout(&self) -> engine::FrameLayoutId {
        self.frame_layout
    }

    /// Return the active MIR block id for this frame.
    pub(crate) fn block_id(
        &self,
        program: &Program,
    ) -> Result<mir::LocalNodeId<mir::Block>, Error> {
        let function = program
            .functions
            .function_by_id(self.function)
            .ok_or(Error::undefined_function(self.function))?;
        let block = function
            .blocks
            .get(self.block as usize)
            .ok_or(Error::invalid_instruction())?;

        Ok(block.mir_block)
    }

    /// Retarget this frame to one lowered function and stack range.
    pub(crate) fn retarget(
        &mut self,
        function: &Function,
        block: u32,
        stack_offset: usize,
        byte_len: usize,
        base: usize,
    ) {
        self.function = function.mir_function;
        self.frame_layout = function.frame_layout;
        self.block = block;
        self.pc = 0;
        self.stack_offset = stack_offset;
        self.byte_len = byte_len;
        self.base = base;
    }

    /// Extend this frame's stack-owned byte range.
    pub(crate) fn extend_bytes_to(&mut self, stack_end: usize) {
        if stack_end > self.stack_offset {
            self.byte_len = stack_end - self.stack_offset;
        }
    }

    /// Return the native address of this frame's byte range.
    #[inline]
    pub(crate) fn base_address(&self) -> usize {
        self.base
    }

    /// Return this frame's byte range.
    #[inline]
    pub(crate) fn bytes(&self) -> &[u8] {
        // SAFETY: base points at byte_len live bytes in the VM stack arena
        unsafe { std::slice::from_raw_parts(self.base as *const u8, self.byte_len) }
    }

    /// Return this frame's byte range mutably.
    #[inline]
    pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: &mut self guarantees exclusive access to this live VM stack frame
        unsafe { std::slice::from_raw_parts_mut(self.base as *mut u8, self.byte_len) }
    }

    /// Return a pointer to one frame slot.
    #[inline(always)]
    pub(crate) fn slot_address(&self, slot: &engine::FrameSlot) -> usize {
        self.base_address() + slot.offset as usize
    }

    /// Read one word from a byte offset.
    #[inline(always)]
    pub(crate) fn read_word_at(&self, offset: u32) -> Word {
        let address = self.base_address() + offset as usize;
        debug_assert_eq!((address % mem::align_of::<Word>()), 0);

        // SAFETY: lowered frame offsets are word-aligned and point inside this frame
        unsafe { std::ptr::read(address as *const Word) }
    }

    /// Write one word into a byte offset.
    #[inline(always)]
    pub(crate) fn write_word_at(&mut self, offset: u32, value: Word) {
        let address = self.base_address() + offset as usize;
        debug_assert_eq!((address % mem::align_of::<Word>()), 0);

        // SAFETY: lowered frame offsets are word-aligned and point inside this frame
        unsafe {
            std::ptr::write(address as *mut Word, value);
        }
    }

    /// Read one word from a slot.
    #[inline(always)]
    pub(crate) fn read_word(&self, slot: &engine::FrameSlot) -> Word {
        debug_assert!(slot.byte_len as usize >= Word::BYTE_LEN);
        debug_assert_eq!((self.slot_address(slot) % mem::align_of::<Word>()), 0);

        // SAFETY: frame slots are lowered as word-sized aligned storage inside this frame
        unsafe { std::ptr::read(self.slot_address(slot) as *const Word) }
    }

    /// Write one word into a slot.
    #[inline(always)]
    pub(crate) fn write_word(&mut self, slot: &engine::FrameSlot, value: Word) {
        debug_assert!(slot.byte_len as usize >= Word::BYTE_LEN);
        debug_assert_eq!((self.slot_address(slot) % mem::align_of::<Word>()), 0);

        // SAFETY: frame slots are lowered as word-sized aligned storage inside this frame
        unsafe {
            std::ptr::write(self.slot_address(slot) as *mut Word, value);
        }
    }

    /// Borrow one slot byte range.
    pub(crate) fn slot_bytes(&self, slot: &engine::FrameSlot) -> &[u8] {
        let start = slot.offset as usize;
        let end = start + slot.byte_len as usize;

        &self.bytes()[start..end]
    }

    /// Borrow one slot byte range mutably.
    pub(crate) fn slot_bytes_mut(&mut self, slot: &engine::FrameSlot) -> &mut [u8] {
        let start = slot.offset as usize;
        let end = start + slot.byte_len as usize;

        &mut self.bytes_mut()[start..end]
    }

    /// Return the address of one local value.
    pub(crate) fn local_address(
        &self,
        layout: &engine::FrameLayout,
        local: mir::LocalNodeId<mir::Local>,
    ) -> Result<usize, Error> {
        let slot = layout
            .local(local.id)
            .ok_or(Error::undefined_local(local))?;

        Ok(self.slot_address(slot))
    }

    /// Return the closure environment for this frame.
    pub(crate) fn load_environment(
        &self,
        layout: &engine::FrameLayout,
    ) -> Result<Option<Word>, Error> {
        let Some(slot) = layout.environment() else {
            return Ok(None);
        };

        Ok(Some(self.read_word(slot)))
    }

    /// Store the closure environment for this frame.
    pub(crate) fn store_environment(
        &mut self,
        layout: &engine::FrameLayout,
        value: Option<Word>,
    ) -> Result<(), Error> {
        let Some(slot) = layout.environment() else {
            return Ok(());
        };
        let value = value.ok_or(Error::invalid_instruction())?;

        self.write_word(slot, value);

        Ok(())
    }

    /// Clear all values (but keep locals).
    pub(crate) fn clear_values(&mut self, layout: &engine::FrameLayout) {
        for slot in layout.values() {
            self.slot_bytes_mut(slot).fill(0);
        }
    }

    /// Return whether this frame owns one stack byte range.
    pub(crate) fn owns_stack_range(&self, address: usize, byte_len: usize) -> bool {
        let start = self.base_address();
        let end = address + byte_len;
        let stack_end = start + self.byte_len;

        start <= address && end <= stack_end
    }

    /// Fork this frame over one already forked stack address.
    pub(crate) fn fork(&self, base: usize) -> Self {
        Self {
            function: self.function,
            frame_layout: self.frame_layout,
            block: self.block,
            pc: self.pc,
            return_state: self.return_state,
            stack_offset: self.stack_offset,
            byte_len: self.byte_len,
            base,
        }
    }

    /// Capture one immutable frame image.
    pub(crate) fn image(&self, program: &Program) -> RuntimeResult<FrameImage> {
        let block = self.block_id(program).map_err(RuntimeError::new)?;
        let point = ProgramPoint::new(self.function(), block, self.pc as u32);
        let frame_state = program.frame_state_at(point).ok_or_else(|| {
            RuntimeError::new(Error::internal(format!(
                "missing frame state for image point: {point:?}"
            )))
        })?;
        program
            .mir_point_for_frame_state(frame_state)
            .ok_or_else(|| {
                RuntimeError::new(Error::internal(format!(
                    "missing MIR point for frame state: {frame_state:?}"
                )))
            })?;

        Ok(FrameImage {
            frame_state,
            return_state: self.return_state,
            stack_offset: self.stack_offset,
            byte_len: self.byte_len,
        })
    }

    /// Create one frame from an immutable image.
    pub(crate) fn from_image(
        image: &FrameImage,
        program: &Program,
        stack_offset: usize,
        base: usize,
    ) -> RuntimeResult<Self> {
        let point = program
            .point_for_frame_state(image.frame_state)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;

        // resolve the lowered function for this frame
        let function_ref = program
            .functions
            .function_by_id(point.function)
            .ok_or_else(|| RuntimeError::new(Error::undefined_function(point.function)))?;

        // resolve the captured block index from the lowered function
        let block = function_ref
            .blocks
            .iter()
            .position(|block| block.mir_block == point.block)
            .ok_or_else(|| RuntimeError::new(Error::undefined_block(point.block)))?;
        let layout_id = function_ref.frame_layout;
        let layout = program
            .frame_layout_by_id(layout_id)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        if image.byte_len < layout.byte_len as usize {
            return Err(RuntimeError::new(Error::invalid_instruction()));
        }

        Ok(Self {
            function: function_ref.mir_function,
            frame_layout: function_ref.frame_layout,
            block: block as u32,
            pc: point.pc as usize,
            return_state: image.return_state,
            stack_offset,
            byte_len: image.byte_len,
            base,
        })
    }
}
