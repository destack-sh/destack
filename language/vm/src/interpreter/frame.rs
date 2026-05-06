use std::mem;
use std::ptr::NonNull;

use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::program::{Function, FunctionTable, Program};
use crate::{RootSink, Word};
use destack_heap::{HeapResult, RootSlot};

/// Call frame in the interpreter.
#[derive(Debug)]
pub struct Frame {
    /// The logical frame layout.
    pub(crate) frame_layout: engine::FrameLayoutId,
    /// Pointer to the lowered function.
    pub(crate) function_ptr: NonNull<Function>,
    /// The current block index in the lowered function.
    pub(crate) block: u32,
    /// Program counter within the current block.
    pub(crate) pc: usize,
    /// The active exceptional call owned by this frame while one callee runs.
    pub(crate) exceptional_call: Option<ExceptionalCall>,
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
    /// The logical frame layout.
    pub frame_layout: engine::FrameLayoutId,
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The current block being executed.
    pub current_block: mir::LocalNodeId<mir::Block>,
    /// The program counter within the current block.
    pub pc: usize,
    /// The active exceptional call owned by this frame while one callee runs.
    pub exceptional_call: Option<ExceptionalCall>,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}

// frame should fit in 64 bytes
const _: () = assert!(std::mem::size_of::<Frame>() <= 64);

// the raw function pointer always points into immutable program data
// that stays alive for the duration of the owning isolate
unsafe impl Send for Frame {}

impl Frame {
    /// Create a new frame for a function.
    pub(crate) fn new(
        frame_layout: engine::FrameLayoutId,
        function_ptr: NonNull<Function>,
        block: u32,
        layout: &engine::FrameLayout,
        stack_offset: usize,
        base: *mut u8,
    ) -> Self {
        Self {
            frame_layout,
            function_ptr,
            block,
            pc: 0,
            exceptional_call: None,
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

    /// Return the current MIR block id.
    #[inline(always)]
    pub(crate) fn current_block(&self) -> mir::LocalNodeId<mir::Block> {
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

    /// Truncate this frame's stack-owned byte range.
    pub(crate) fn truncate_bytes_to(&mut self, stack_end: usize) {
        debug_assert!(stack_end >= self.stack_offset);
        self.byte_len = stack_end - self.stack_offset;
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
    pub(crate) fn environment(&self, layout: &engine::FrameLayout) -> Result<Option<Word>, Error> {
        let Some(slot) = layout.environment() else {
            return Ok(None);
        };

        Ok(Some(self.read_word(slot)))
    }

    /// Store the callable environment for this frame.
    pub(crate) fn set_environment(&mut self, layout: &engine::FrameLayout, value: Option<Word>) {
        let Some(slot) = layout.environment() else {
            return;
        };

        self.write_word(slot, value.unwrap_or(Word::VOID));
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

    /// Visit all heap references from this frame for GC roots.
    pub(crate) fn visit_roots(
        &self,
        program: &Program,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let function = self.function();
        let layout = program
            .frame_layout(function)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing frame layout for frame: {function:?}"),
            })?;

        // ssa values
        for slot in layout.values() {
            self.visit_slot_roots(program, slot, roots)?;
        }

        // locals
        for slot in layout.locals() {
            self.visit_slot_roots(program, slot, roots)?;
        }

        // callable environment
        if let Some(slot) = layout.environment() {
            self.visit_slot_roots(program, slot, roots)?;
        }

        Ok(())
    }

    /// Visit mutable local root slots in this frame.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let function = self.function();
        let layout = program
            .frame_layout(function)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing frame layout for frame: {function:?}"),
            })?;

        // ssa values
        for slot in layout.values() {
            self.visit_slot_root_slots(program, slot, visit)?;
        }

        // locals
        for slot in layout.locals() {
            self.visit_slot_root_slots(program, slot, visit)?;
        }

        // callable environment
        if let Some(slot) = layout.environment() {
            self.visit_slot_root_slots(program, slot, visit)?;
        }

        Ok(())
    }

    /// Visit heap roots materialized at one safepoint.
    pub(crate) fn visit_materialized_roots(
        &self,
        program: &Program,
        materialization: &engine::FrameMaterialization,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let layout = program
            .frame_layout_by_id(materialization.frame_layout)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing materialized frame layout for root scan: {:?}",
                    materialization.frame_layout
                ),
            })?;

        visit_materialized_slots(layout, materialization, |slot| {
            self.visit_slot_roots(program, slot, roots)
        })
    }

    /// Visit mutable local root slots materialized at one safepoint.
    pub(crate) fn visit_materialized_root_slots(
        &mut self,
        program: &Program,
        materialization: &engine::FrameMaterialization,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = program
            .frame_layout_by_id(materialization.frame_layout)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!(
                    "missing materialized frame layout for heap roots: {:?}",
                    materialization.frame_layout
                ),
            })?;

        visit_materialized_slots(layout, materialization, |slot| {
            self.visit_slot_root_slots(program, slot, visit)
        })
    }

    /// Visit heap roots stored in one frame slot.
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
                        "missing frame slot layout for root scan: layout={:?}",
                        slot.layout
                    ),
                })?;
        if !layout.is_word() {
            return program.visit_byte_roots(
                program.type_for_layout(slot.layout),
                self.slot_bytes(slot),
                roots,
            );
        }

        program.visit_value_root(
            program.type_for_layout(slot.layout),
            self.read_word(slot),
            roots,
        )
    }

    /// Visit local root slots stored in one frame slot.
    fn visit_slot_root_slots(
        &mut self,
        program: &Program,
        slot: &engine::FrameSlot,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout =
            program
                .layout_for_layout(slot.layout)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!(
                        "missing frame slot layout for heap roots: layout={:?}",
                        slot.layout
                    ),
                })?;
        if !layout.is_word() {
            return program.visit_byte_root_slots(
                program.type_for_layout(slot.layout),
                self.slot_bytes_mut(slot),
                visit,
            );
        }

        if program.is_local_root_type(program.type_for_layout(slot.layout))? {
            visit(RootSlot::Bytes(self.slot_bytes_mut(slot))).map_err(Error::from)?;
        }

        Ok(())
    }

    /// Clone this frame over one already forked stack address.
    pub(crate) fn clone_for_fork(&self, base: *mut u8) -> Self {
        Self {
            frame_layout: self.frame_layout,
            function_ptr: self.function_ptr,
            block: self.block,
            pc: self.pc,
            exceptional_call: self.exceptional_call.clone(),
            stack_offset: self.stack_offset,
            byte_len: self.byte_len,
            base,
        }
    }

    /// Capture one immutable frame image.
    pub(crate) fn image(&self) -> FrameImage {
        FrameImage {
            frame_layout: self.frame_layout,
            function: self.function(),
            current_block: self.current_block(),
            pc: self.pc,
            exceptional_call: self.exceptional_call.clone(),
            bytes: self.bytes().to_vec(),
        }
    }

    /// Create one frame from an immutable image.
    pub(crate) fn from_image(
        image: &FrameImage,
        functions: &FunctionTable,
        layout: &engine::FrameLayout,
        stack_offset: usize,
        base: *mut u8,
    ) -> RuntimeResult<Self> {
        // resolve the lowered function for this frame
        let function_index = functions.index_for(image.function).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: image.function,
            })
        })?;

        let function_ptr = functions.pointer(function_index).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: image.function,
            })
        })?;

        // resolve the current block index from the lowered function
        let function_ref = unsafe { function_ptr.as_ref() };
        let block = function_ref
            .blocks
            .iter()
            .position(|block| block.mir_block == image.current_block)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedBlock {
                    block: image.current_block,
                })
            })?;

        Ok(Self {
            frame_layout: image.frame_layout,
            function_ptr,
            block: block as u32,
            pc: image.pc,
            exceptional_call: image.exceptional_call.clone(),
            stack_offset,
            byte_len: layout.byte_len as usize,
            base,
        })
    }
}

/// One active exceptional call parked on a caller frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionalCall {
    /// The frame state to enter when the callee returns normally.
    pub(crate) normal_state: engine::FrameStateId,
    /// The frame state to enter when the callee throws.
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Visit each materialized frame slot once.
pub(crate) fn visit_materialized_slots(
    layout: &engine::FrameLayout,
    materialization: &engine::FrameMaterialization,
    mut visit: impl FnMut(&engine::FrameSlot) -> Result<(), Error>,
) -> Result<(), Error> {
    // resolve copied source ids into VM frame slots
    for slot in materialization.copied_slots() {
        let slot = layout.slot(slot).ok_or(Error::InvalidContinuation)?;
        visit(slot)?;
    }

    Ok(())
}
