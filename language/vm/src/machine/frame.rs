use std::{mem, ptr};

use destack_mir as mir;

use super::Activation;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::{Cell, FramePointer};
use destack_program::vm::{ArgumentRange, Function, ProgramPoint};
use destack_program::{FrameImage, FunctionId, Program};

/// Call frame in the VM machine.
///
/// The byte address is owned by the page-backed VM stack.
#[derive(Debug)]
pub struct Frame {
    /// Current function id.
    pub(crate) function: FunctionId,
    /// The logical frame layout id.
    pub(crate) frame_layout: mir::FrameLayoutId,
    /// The current block index in the lowered function.
    pub(crate) block: u32,
    /// Program counter within the current block.
    pub(crate) pc: usize,
    /// The caller frame state after one callee returns.
    pub(crate) return_state: Option<mir::FrameStateId>,
    /// The byte offset in the machine stack arena.
    pub(crate) stack_offset: usize,
    /// The frame byte width.
    pub(crate) byte_len: usize,
    /// Native address of the frame bytes in the machine stack arena.
    base: usize,
}

// frame should fit in 64 bytes
const _: () = assert!(std::mem::size_of::<Frame>() <= 64);

impl Frame {
    /// Create a new frame for a function.
    pub(crate) fn new(
        function: &Function,
        block: u32,
        layout: &mir::FrameLayout,
        stack_offset: usize,
        base: usize,
    ) -> Self {
        Self {
            function: function.function,
            frame_layout: function.frame_layout,
            block,
            pc: 0,
            return_state: None,
            stack_offset,
            byte_len: layout.byte_len as usize,
            base,
        }
    }

    /// Return the current function id.
    #[inline(always)]
    pub(crate) fn function(&self) -> FunctionId {
        self.function
    }

    /// Return the logical frame layout id.
    #[inline(always)]
    pub(crate) fn frame_layout(&self) -> mir::FrameLayoutId {
        self.frame_layout
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
        self.function = function.function;
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
    pub(crate) fn slot_address(&self, slot: &mir::FrameSlot) -> usize {
        self.base_address() + slot.offset as usize
    }

    /// Read one cell from a byte offset.
    #[inline(always)]
    pub(crate) fn read_cell_at(&self, offset: u32) -> Cell {
        let address = self.base_address() + offset as usize;
        debug_assert_eq!((address % mem::align_of::<Cell>()), 0);

        // SAFETY: lowered frame offsets are cell-aligned and point inside this frame
        unsafe { std::ptr::read(address as *const Cell) }
    }

    /// Write one cell into a byte offset.
    #[inline(always)]
    pub(crate) fn write_cell_at(&mut self, offset: u32, value: Cell) {
        let address = self.base_address() + offset as usize;
        debug_assert_eq!((address % mem::align_of::<Cell>()), 0);

        // SAFETY: lowered frame offsets are cell-aligned and point inside this frame
        unsafe {
            std::ptr::write(address as *mut Cell, value);
        }
    }

    /// Read one cell from a slot.
    #[inline(always)]
    pub(crate) fn read_cell(&self, slot: &mir::FrameSlot) -> Cell {
        debug_assert!(slot.byte_len as usize >= Cell::BYTE_LEN);
        debug_assert_eq!((self.slot_address(slot) % mem::align_of::<Cell>()), 0);

        // SAFETY: frame slots are lowered as cell-sized aligned storage inside this frame
        unsafe { std::ptr::read(self.slot_address(slot) as *const Cell) }
    }

    /// Write one cell into a slot.
    #[inline(always)]
    pub(crate) fn write_cell(&mut self, slot: &mir::FrameSlot, value: Cell) {
        debug_assert!(slot.byte_len as usize >= Cell::BYTE_LEN);
        debug_assert_eq!((self.slot_address(slot) % mem::align_of::<Cell>()), 0);

        // SAFETY: frame slots are lowered as cell-sized aligned storage inside this frame
        unsafe {
            std::ptr::write(self.slot_address(slot) as *mut Cell, value);
        }
    }

    /// Borrow one slot byte range.
    pub(crate) fn slot_bytes(&self, slot: &mir::FrameSlot) -> &[u8] {
        let start = slot.offset as usize;
        let end = start + slot.byte_len as usize;

        &self.bytes()[start..end]
    }

    /// Borrow one slot byte range mutably.
    pub(crate) fn slot_bytes_mut(&mut self, slot: &mir::FrameSlot) -> &mut [u8] {
        let start = slot.offset as usize;
        let end = start + slot.byte_len as usize;

        &mut self.bytes_mut()[start..end]
    }

    /// Return the address of one local value.
    pub(crate) fn local_address(
        &self,
        layout: &mir::FrameLayout,
        local: mir::LocalNodeId<mir::Local>,
    ) -> Result<usize, Error> {
        let slot = layout
            .local(local.id)
            .ok_or(Error::undefined_local(local))?;

        Ok(self.slot_address(slot))
    }

    /// Return the function environment for this frame.
    pub(crate) fn load_environment(
        &self,
        layout: &mir::FrameLayout,
    ) -> Result<Option<Cell>, Error> {
        let Some(slot) = layout.environment() else {
            return Ok(None);
        };

        Ok(Some(self.read_cell(slot)))
    }

    /// Store the function environment for this frame.
    pub(crate) fn store_environment(
        &mut self,
        layout: &mir::FrameLayout,
        value: Option<Cell>,
    ) -> Result<(), Error> {
        let Some(slot) = layout.environment() else {
            return Ok(());
        };
        let value = value.ok_or(Error::invalid_instruction())?;

        self.write_cell(slot, value);

        Ok(())
    }

    /// Clear all values (but keep locals).
    pub(crate) fn clear_values(&mut self, layout: &mir::FrameLayout) {
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
        let point = ProgramPoint::new(self.function(), self.block, self.pc as u32);
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
            .vm_functions()
            .function_by_id(point.function)
            .ok_or_else(|| RuntimeError::new(Error::undefined_function(point.function)))?;

        let layout_id = function_ref.frame_layout;
        let layout = program
            .frame_layout_by_id(layout_id)
            .ok_or_else(|| RuntimeError::new(Error::invalid_instruction()))?;
        if image.byte_len < layout.byte_len as usize {
            return Err(RuntimeError::new(Error::invalid_instruction()));
        }

        Ok(Self {
            function: function_ref.function,
            frame_layout: function_ref.frame_layout,
            block: point.block,
            pc: point.pc as usize,
            return_state: image.return_state,
            stack_offset,
            byte_len: image.byte_len,
            base,
        })
    }
}

impl Activation<'_> {
    /// Return the MIR type stored in one SSA value.
    #[inline]
    pub(crate) fn value_type(
        &self,
        value: mir::Value,
    ) -> Result<mir::LocalNodeId<mir::Type>, Error> {
        let slot = self
            .frame_layout()
            .value(value.0)
            .ok_or(Error::invalid_instruction())?;

        Ok(slot.ty)
    }

    /// Return the frame slot for one SSA value.
    #[inline]
    pub(crate) fn value_slot(&self, value: mir::Value) -> Result<&mir::FrameSlot, Error> {
        self.frame_layout()
            .value(value.0)
            .ok_or(Error::invalid_instruction())
    }

    /// Return whether one SSA value is stored as one cell.
    #[inline]
    pub(crate) fn value_is_cell(&self, value: mir::Value) -> Result<bool, Error> {
        let slot = self.value_slot(value)?;

        Ok(self.machine.program.frame_slot_is_cell(slot))
    }

    /// Borrow one SSA value's bytes.
    #[inline]
    pub(crate) fn value_bytes(&self, value: mir::Value) -> Result<&[u8], Error> {
        let slot = self.value_slot(value)?;
        let frame = self.active_frame();

        Ok(frame.slot_bytes(slot))
    }

    /// Borrow one SSA value's bytes mutably.
    #[inline]
    pub(crate) fn value_bytes_mut(&mut self, value: mir::Value) -> Result<&mut [u8], Error> {
        let slot = self.value_slot(value)?.clone();
        let frame = self.active_frame_mut();

        Ok(frame.slot_bytes_mut(&slot))
    }

    /// Move the machine to another live frame.
    pub(crate) fn enter_frame(&mut self, frame_index: usize) -> Result<(), Error> {
        self.bind_frame(frame_index)
    }

    /// Borrow the active frame mutably.
    #[inline(always)]
    pub(crate) fn active_frame_mut(&mut self) -> &mut Frame {
        let frame_index = self.frame_index;
        debug_assert!(frame_index < self.machine.frames.len());

        // SAFETY: activation frame binding validates the active frame index
        unsafe { self.machine.frames.get_unchecked_mut(frame_index) }
    }

    /// Borrow the active frame.
    #[inline(always)]
    pub(crate) fn active_frame(&self) -> &Frame {
        let frame_index = self.frame_index;
        debug_assert!(frame_index < self.machine.frames.len());

        // SAFETY: activation frame binding validates the active frame index
        unsafe { self.machine.frames.get_unchecked(frame_index) }
    }

    /// Borrow the current frame layout.
    #[inline(always)]
    pub(crate) fn frame_layout(&self) -> &mir::FrameLayout {
        let layout = self.machine.program.frame_layout_by_id(self.frame_layout);
        debug_assert!(layout.is_some());

        // SAFETY: active frames are created only from compiled frame layouts
        unsafe { layout.unwrap_unchecked() }
    }

    /// Borrow one frame.
    #[inline(always)]
    pub(crate) fn frame(&self, frame_index: usize) -> Result<&Frame, Error> {
        self.machine
            .frames
            .get(frame_index)
            .ok_or(Error::invalid_instruction())
    }

    /// Allocate zeroed bytes owned by the current frame.
    pub(crate) fn allocate_stack_zeroed(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> Result<usize, Error> {
        let base = self
            .machine
            .stack
            .allocate_zeroed(byte_len, alignment)
            .map_err(|_| Error::stack_overflow())?;

        self.stack_address(base, byte_len)
    }

    /// Allocate uninitialized bytes owned by the current frame.
    pub(crate) fn allocate_stack_uninit(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> Result<usize, Error> {
        let base = self
            .machine
            .stack
            .allocate_uninit(byte_len, alignment)
            .map_err(|_| Error::stack_overflow())?;

        self.stack_address(base, byte_len)
    }

    /// Return the checked address for one newly allocated stack range.
    fn stack_address(&mut self, base: usize, byte_len: usize) -> Result<usize, Error> {
        let end = self.machine.stack.len();
        self.active_frame_mut().extend_bytes_to(end);
        let address = self
            .machine
            .stack
            .address(base, byte_len)
            .map_err(|_| Error::stack_overflow())?;

        Ok(address)
    }

    /// Load one SSA value as a VM cell.
    #[inline(always)]
    pub(crate) fn load_value(&self, v: mir::Value) -> Cell {
        let slot = self.value_slot_unchecked(v);

        // cell values live inline in the frame
        if self.machine.program.frame_slot_is_cell(slot) {
            return self.read_frame_cell(slot.offset);
        }

        // aggregate SSA values are represented by their frame address
        Cell::frame_pointer(FramePointer::from_address(
            self.frame_base + slot.offset as usize,
        ))
    }

    /// Read one cell by frame byte offset.
    #[inline(always)]
    pub(crate) fn load_cell_at(&self, offset: u32) -> Cell {
        self.read_frame_cell(offset)
    }

    /// Return one frame pointer by frame byte offset.
    #[inline(always)]
    pub(crate) fn frame_pointer_at(&self, offset: u32) -> FramePointer {
        let address = self.frame_base + offset as usize;

        FramePointer::from_address(address)
    }

    /// Borrow frame bytes at one byte offset.
    #[inline(always)]
    pub(crate) fn frame_bytes_at(&self, offset: u32, byte_len: usize) -> &[u8] {
        let address = self.frame_base + offset as usize;

        // SAFETY: lowered frame offsets point inside the active frame layout
        unsafe { std::slice::from_raw_parts(address as *const u8, byte_len) }
    }

    /// Borrow frame bytes while mutating the machine.
    #[inline(always)]
    pub(crate) fn with_frame_bytes_at<T>(
        &mut self,
        offset: u32,
        byte_len: usize,
        operation: impl FnOnce(&mut Self, &[u8]) -> T,
    ) -> T {
        let address = self.frame_base + offset as usize;

        // SAFETY: lowered frame offsets point inside the active frame layout
        unsafe {
            let bytes = std::slice::from_raw_parts(address as *const u8, byte_len);

            operation(self, bytes)
        }
    }

    /// Store frame bytes at one byte offset.
    #[inline(always)]
    pub(crate) fn store_frame_bytes_at(&mut self, offset: u32, bytes: &[u8]) {
        let address = self.frame_base + offset as usize;

        // SAFETY: lowered frame offsets point inside the active frame layout
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
        }
    }

    /// Copy frame bytes into one native address.
    #[inline(always)]
    pub(crate) fn copy_frame_bytes_to_address(
        &self,
        source: u32,
        destination: usize,
        byte_len: usize,
    ) {
        let source = self.frame_base + source as usize;

        // SAFETY: caller provides a valid destination and lower validates the source frame range
        unsafe {
            ptr::copy(source as *const u8, destination as *mut u8, byte_len);
        }
    }

    /// Read one aligned cell from the current frame.
    #[inline(always)]
    fn read_frame_cell(&self, offset: u32) -> Cell {
        let address = self.frame_base + offset as usize;
        debug_assert_eq!(address % mem::align_of::<Cell>(), 0);

        // SAFETY: lowered cell offsets are cell-aligned and point inside the active frame
        unsafe { ptr::read(address as *const Cell) }
    }

    /// Return the frame slot for one SSA value without bounds checks.
    #[inline(always)]
    fn value_slot_unchecked(&self, v: mir::Value) -> &mir::FrameSlot {
        let index = v.0 as usize;
        let layout = self.frame_layout();
        debug_assert!(
            index < layout.values().len(),
            "ssa value out of bounds: {v:?}"
        );

        // SAFETY: lower only emits SSA values present in the active frame layout
        unsafe { layout.values().get_unchecked(index) }
    }

    /// Store one cell into an SSA value.
    #[inline(always)]
    pub(crate) fn store_value_cell(&mut self, v: mir::Value, val: Cell) {
        let slot = self.value_slot_unchecked(v);
        let is_cell = self.machine.program.frame_slot_is_cell(slot);
        let offset = slot.offset;

        debug_assert!(is_cell, "attempted cell write into frame bytes");
        self.write_frame_cell(offset, val);
    }

    /// Write one cell by frame byte offset.
    #[inline(always)]
    pub(crate) fn store_cell_at(&mut self, offset: u32, val: Cell) {
        self.write_frame_cell(offset, val);
    }

    /// Write one aligned cell into the current frame.
    #[inline(always)]
    fn write_frame_cell(&mut self, offset: u32, value: Cell) {
        let address = self.frame_base + offset as usize;
        debug_assert_eq!(address % mem::align_of::<Cell>(), 0);

        // SAFETY: lowered cell offsets are cell-aligned and point inside the active frame
        unsafe {
            ptr::write(address as *mut Cell, value);
        }
    }

    /// Copy one byte range inside the current frame.
    #[inline(always)]
    pub(crate) fn copy_frame_bytes(
        &mut self,
        source_offset: u32,
        destination_offset: u32,
        byte_len: usize,
    ) {
        let source_offset = source_offset as usize;
        let destination_offset = destination_offset as usize;

        // SAFETY: lowered frame offsets point inside the active frame layout
        unsafe {
            ptr::copy(
                (self.frame_base + source_offset) as *const u8,
                (self.frame_base + destination_offset) as *mut u8,
                byte_len,
            );
        }
    }

    /// Return the argument slice for the given range.
    #[inline(always)]
    pub(crate) fn argument_slice(&self, range: ArgumentRange) -> &[mir::Value] {
        let function = self
            .machine
            .program
            .vm_functions()
            .function_by_id(self.active_frame().function());
        debug_assert!(function.is_some());
        // SAFETY: active frames are created only from lowered program functions
        let function = unsafe { function.unwrap_unchecked() };
        let start = range.start as usize;
        let len = range.len as usize;
        let end = start + len;
        debug_assert!(
            end <= function.argument_pool.len(),
            "argument pool out of bounds for range"
        );

        &function.argument_pool[start..end]
    }
}
