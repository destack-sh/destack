use std::mem;
use std::ptr::NonNull;

use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_mir as mir};

use crate::Word;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::program::{Function, Program, ProgramPoint};

/// Call frame in the interpreter.
///
/// The raw byte pointer is owned by the page-backed VM stack.
#[derive(Debug)]
pub struct Frame {
    /// Pointer to the lowered function.
    pub(crate) function_ptr: NonNull<Function>,
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
    /// Pointer to the frame bytes in the interpreter stack arena.
    base: *mut u8,
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

// the raw function pointer always points into immutable program data
// that stays alive for the duration of the owning isolate
unsafe impl Send for Frame {}

impl Frame {
    /// Create a new frame for a function.
    pub(crate) fn new(
        function_ptr: NonNull<Function>,
        block: u32,
        layout: &engine::FrameLayout,
        stack_offset: usize,
        base: *mut u8,
    ) -> Self {
        Self {
            function_ptr,
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
        unsafe { self.function_ptr.as_ref().mir_function }
    }

    /// Return the logical frame layout id.
    #[inline(always)]
    pub(crate) fn frame_layout(&self) -> engine::FrameLayoutId {
        unsafe { self.function_ptr.as_ref().frame_layout }
    }

    /// Return the active MIR block id.
    #[inline(always)]
    pub(crate) fn block_id(&self) -> mir::LocalNodeId<mir::Block> {
        unsafe { self.function_ptr.as_ref().blocks[self.block as usize].mir_block }
    }

    /// Replace this frame's byte range.
    pub(crate) fn replace_bytes(&mut self, stack_offset: usize, byte_len: usize, base: *mut u8) {
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
        self.base as usize
    }

    /// Return this frame's byte range.
    #[inline]
    pub(crate) fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.base, self.byte_len) }
    }

    /// Return this frame's byte range mutably.
    #[inline]
    pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.base, self.byte_len) }
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

        unsafe { std::ptr::read(address as *const Word) }
    }

    /// Write one word into a byte offset.
    #[inline(always)]
    pub(crate) fn write_word_at(&mut self, offset: u32, value: Word) {
        let address = self.base_address() + offset as usize;
        debug_assert_eq!((address % mem::align_of::<Word>()), 0);

        unsafe {
            std::ptr::write(address as *mut Word, value);
        }
    }

    /// Read one word from a slot.
    #[inline(always)]
    pub(crate) fn read_word(&self, slot: &engine::FrameSlot) -> Word {
        debug_assert!(slot.byte_len as usize >= Word::BYTE_LEN);
        debug_assert_eq!((self.slot_address(slot) % mem::align_of::<Word>()), 0);

        unsafe { std::ptr::read(self.slot_address(slot) as *const Word) }
    }

    /// Write one word into a slot.
    #[inline(always)]
    pub(crate) fn write_word(&mut self, slot: &engine::FrameSlot, value: Word) {
        debug_assert!(slot.byte_len as usize >= Word::BYTE_LEN);
        debug_assert_eq!((self.slot_address(slot) % mem::align_of::<Word>()), 0);

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

    /// Write one word into a scalar SSA value.
    #[inline]
    pub(crate) fn write_value_word(
        &mut self,
        layout: &engine::FrameLayout,
        value: mir::Value,
        word: Word,
    ) -> Result<(), Error> {
        let slot = layout
            .value(value.0)
            .ok_or(Error::UndefinedValue { value })?;
        if !slot.is_word {
            return Err(Error::TypeMismatch {
                expected: "word value".to_string(),
                actual: format!("frame-backed value: {value:?}"),
            });
        }

        self.write_word(slot, word);

        Ok(())
    }

    /// Return the address of one local value.
    pub(crate) fn local_address(
        &self,
        layout: &engine::FrameLayout,
        local: mir::LocalNodeId<mir::Local>,
    ) -> Result<usize, Error> {
        let slot = layout
            .local(local.id)
            .ok_or(Error::UndefinedLocal { local })?;

        Ok(self.slot_address(slot))
    }

    /// Return the callable environment for this frame.
    pub(crate) fn load_environment(
        &self,
        layout: &engine::FrameLayout,
    ) -> Result<Option<Word>, Error> {
        let Some(slot) = layout.environment() else {
            return Ok(None);
        };

        Ok(Some(self.read_word(slot)))
    }

    /// Store the callable environment for this frame.
    pub(crate) fn store_environment(
        &mut self,
        layout: &engine::FrameLayout,
        value: Option<Word>,
    ) -> Result<(), Error> {
        let Some(slot) = layout.environment() else {
            return Ok(());
        };
        let value = value.ok_or(Error::InvalidInstruction)?;

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

    /// Clone this frame over one already forked stack address.
    pub(crate) fn clone_for_fork(&self, base: *mut u8) -> Self {
        Self {
            function_ptr: self.function_ptr,
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
        let point = ProgramPoint::new(self.function(), self.block_id(), self.pc as u32);
        let frame_state = program.frame_state_at(point).ok_or_else(|| {
            RuntimeError::new(Error::InvariantViolation {
                context: format!("missing frame state for image point: {point:?}"),
            })
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
        base: *mut u8,
    ) -> RuntimeResult<Self> {
        let point = program
            .point_for_frame_state(image.frame_state)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;

        // resolve the lowered function for this frame
        let function_index = program
            .functions
            .local_index(point.function)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedFunction {
                    function: point.function,
                })
            })?;

        let function_ptr = program
            .functions
            .pointer_by_index(function_index)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedFunction {
                    function: point.function,
                })
            })?;

        // resolve the captured block index from the lowered function
        let function_ref = unsafe { function_ptr.as_ref() };
        let block = function_ref
            .blocks
            .iter()
            .position(|block| block.mir_block == point.block)
            .ok_or_else(|| RuntimeError::new(Error::UndefinedBlock { block: point.block }))?;
        let layout_id = unsafe { function_ptr.as_ref().frame_layout };
        let layout = program
            .frame_layout_by_id(layout_id)
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
        if image.byte_len < layout.byte_len as usize {
            return Err(RuntimeError::new(Error::InvalidInstruction));
        }

        Ok(Self {
            function_ptr,
            block: block as u32,
            pc: point.instruction_index as usize,
            return_state: image.return_state,
            stack_offset,
            byte_len: image.byte_len,
            base,
        })
    }
}
