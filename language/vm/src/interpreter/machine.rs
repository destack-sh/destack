use std::fmt;

use destack_heap::{
    AllocationLayout as HeapAllocationLayout, AllocationShape, Heap, HeapReference,
    SharedAllocator, SharedGcPhase, SharedGcWorker, SharedHeapReference, SmallAllocationLayout,
};
use engine::StaticSpace;
use {destack_engine as engine, destack_mir as mir};

use super::{Frame, Interpreter};
use crate::diagnostic::{Error, RuntimeError};
use crate::options::IsolateOptions;
use crate::program::{
    ArgumentRange, ElementAccess, ElementAccessId, FieldAccess, FieldAccessId, FrameAccess,
    FrameAccessId, Function, Instruction, Layout, PointeeAccess, PointeeAccessId, Program,
    SideRecord, SideTable, SliceElementAccess, SliceElementAccessId,
};
use crate::{FramePointer, SharedHeap, StackPointer, StaticPointer, Word};

/// Execution context for one interpreter frame.
pub(crate) struct Machine<'ctx, 'iso> {
    /// Program being executed.
    pub(crate) program: &'iso Program,
    /// Immutable isolate options.
    pub(crate) options: &'iso IsolateOptions,
    /// Mutable static byte arena.
    pub(crate) statics: &'iso mut StaticSpace,
    /// The worker-local heap.
    heap: *mut Heap,
    /// The world-shared heap.
    shared: *const SharedHeap,
    /// Shared collector worker for allocation assist.
    shared_gc: &'iso SharedGcWorker,
    /// The worker cache for shared heap allocations.
    shared_allocator: *mut SharedAllocator,
    /// Interpreter owning the live stack.
    pub(crate) interpreter: &'ctx mut Interpreter,

    /// Index of the current frame in the stack.
    pub frame_index: usize,
    /// Whether bounds checks are enabled.
    pub bounds_checks: bool,
    /// Whether null checks are enabled.
    pub null_checks: bool,
    /// Whether reference address-space checks are enabled.
    pub reference_kind_checks: bool,
    /// Whether reference mutability checks are enabled.
    pub reference_mutability_checks: bool,
    /// Pointer to the current frame.
    frame: *mut Frame,
    /// Pointer to the current frame layout.
    frame_layout: *const engine::FrameLayout,
    /// Pointer to the program side table.
    side_table: *const SideTable,
    /// Argument pool for the current function.
    argument_pool: *const mir::Value,
    /// Argument pool length.
    argument_pool_len: usize,
}

impl fmt::Debug for Machine<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Machine")
            .field("frame_index", &self.frame_index)
            .field("argument_pool_len", &self.argument_pool_len)
            .field("bounds_checks", &self.bounds_checks)
            .field("null_checks", &self.null_checks)
            .field("reference_kind_checks", &self.reference_kind_checks)
            .field(
                "reference_mutability_checks",
                &self.reference_mutability_checks,
            )
            .finish()
    }
}

impl<'ctx, 'iso> Machine<'ctx, 'iso> {
    /// Create a machine context for the current frame.
    pub(crate) fn new(
        program: &'iso Program,
        options: &'iso IsolateOptions,
        statics: &'iso mut StaticSpace,
        heap: &'iso mut Heap,
        shared: &'iso SharedHeap,
        shared_gc: &'iso SharedGcWorker,
        shared_allocator: &'iso mut SharedAllocator,
        interpreter: &'ctx mut Interpreter,
        frame_index: usize,
        function: &'iso Function,
    ) -> Result<Self, Error> {
        // resolve check policies
        let bounds_checks = options.checks.bounds;
        let null_checks = options.checks.null;
        let reference_kind_checks = options.checks.reference_kind;
        let reference_mutability_checks = options.checks.reference_mutability;

        // get frame pointer
        debug_assert!(
            frame_index < interpreter.frames.len(),
            "frame index out of bounds"
        );
        // safety: frame_index always points at the current frame
        let frame = unsafe { interpreter.frames.get_unchecked_mut(frame_index) as *mut Frame };

        let frame_layout = unsafe { (*frame).frame_layout };
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameLayout;

        Ok(Self {
            program,
            options,
            statics,
            shared_gc,
            heap: heap as *mut Heap,
            shared: shared as *const SharedHeap,
            shared_allocator: shared_allocator as *mut SharedAllocator,
            interpreter,
            frame_index,
            bounds_checks,
            null_checks,
            reference_kind_checks,
            reference_mutability_checks,
            frame,
            frame_layout,
            side_table: &program.side_table,
            argument_pool: function.argument_pool.as_ptr(),
            argument_pool_len: function.argument_pool.len(),
        })
    }

    /// Borrow the program MIR tree.
    #[inline]
    pub(crate) fn tree(&self) -> &mir::Tree {
        &self.program.tree
    }

    /// Borrow one pooled side record.
    #[inline(always)]
    pub(crate) fn side<T: SideRecord>(&self, instruction: &Instruction) -> &'iso T {
        unsafe { T::get(&*self.side_table, instruction.a) }
    }

    /// Return the compiled layout for one MIR type.
    #[inline]
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&Layout, Error> {
        self.program.layout(ty).ok_or_else(|| Error::TypeMismatch {
            expected: "compiled layout".to_string(),
            actual: format!("{ty:?}"),
        })
    }

    /// Return the MIR type stored in one SSA value.
    #[inline]
    pub(crate) fn value_type(
        &self,
        value: mir::Value,
    ) -> Result<mir::LocalNodeId<mir::Type>, Error> {
        let slot = self
            .frame_layout()
            .value(value.0)
            .ok_or(Error::InvalidInstruction)?;

        Ok(self.program.type_for_layout(slot.layout))
    }

    /// Return the frame slot for one SSA value.
    #[inline]
    pub(crate) fn value_slot(&self, value: mir::Value) -> Result<&engine::FrameSlot, Error> {
        self.frame_layout()
            .value(value.0)
            .ok_or(Error::InvalidInstruction)
    }

    /// Return whether one SSA value is stored as one word.
    #[inline]
    pub(crate) fn value_is_word(&self, value: mir::Value) -> Result<bool, Error> {
        Ok(self.value_slot(value)?.is_word)
    }

    /// Borrow one SSA value's bytes.
    #[inline]
    pub(crate) fn value_bytes(&self, value: mir::Value) -> Result<&[u8], Error> {
        let slot = self.value_slot(value)?;
        let frame = unsafe { &*self.frame };

        Ok(frame.slot_bytes(slot))
    }

    /// Borrow one SSA value's bytes mutably.
    #[inline]
    pub(crate) fn value_bytes_mut(&mut self, value: mir::Value) -> Result<&mut [u8], Error> {
        let slot = self.value_slot(value)? as *const engine::FrameSlot;
        let frame = self.current_frame_mut();

        Ok(frame.slot_bytes_mut(unsafe { &*slot }))
    }

    /// Borrow the isolate options.
    #[inline]
    pub(crate) fn options(&self) -> &IsolateOptions {
        self.options
    }

    /// Create a runtime error with current call stack.
    #[cold]
    pub(crate) fn runtime_error(&self, error: Error) -> RuntimeError {
        self.interpreter.runtime_error(self.program, error)
    }

    /// Refresh cached frame data after moving to another function.
    pub(crate) fn refresh_frame(&mut self, function: &Function) -> Result<(), Error> {
        let frame_layout = unsafe { (*self.frame).frame_layout };
        let frame_layout = self
            .program
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::InvalidInstruction)?;
        self.frame_layout = frame_layout as *const engine::FrameLayout;

        // refresh argument pool
        self.argument_pool = function.argument_pool.as_ptr();
        self.argument_pool_len = function.argument_pool.len();

        Ok(())
    }

    /// Return the program side table pointer.
    #[inline(always)]
    pub(crate) fn side_table_ptr(&self) -> *const SideTable {
        self.side_table
    }

    /// Return one pooled field access.
    #[inline(always)]
    pub(crate) fn field_access(&self, id: FieldAccessId) -> FieldAccess {
        *unsafe { (*self.side_table).field_access(id) }
    }

    /// Return one pooled frame access.
    #[inline(always)]
    pub(crate) fn frame_access(&self, id: FrameAccessId) -> FrameAccess {
        *unsafe { (*self.side_table).frame_access(id) }
    }

    /// Return one pooled element access.
    #[inline(always)]
    pub(crate) fn element_access(&self, id: ElementAccessId) -> ElementAccess {
        *unsafe { (*self.side_table).element_access(id) }
    }

    /// Return one pooled slice element access.
    #[inline(always)]
    pub(crate) fn slice_element_access(&self, id: SliceElementAccessId) -> SliceElementAccess {
        *unsafe { (*self.side_table).slice_element_access(id) }
    }

    /// Return one pooled pointee access.
    #[inline(always)]
    pub(crate) fn pointee_access(&self, id: PointeeAccessId) -> PointeeAccess {
        *unsafe { (*self.side_table).pointee_access(id) }
    }

    /// Borrow the worker heap mutably.
    #[inline]
    pub(crate) fn heap_mut(&mut self) -> &mut Heap {
        unsafe { &mut *self.heap }
    }

    /// Allocate one zeroed local heap payload from one allocation shape.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_shape(
        &mut self,
        shape: AllocationShape<'_>,
    ) -> Result<HeapReference, Error> {
        let heap = unsafe { &mut *self.heap };
        let layout = heap.allocation_layout(shape);

        heap.allocate_zeroed(&layout).map_err(Error::from)
    }

    /// Allocate one zeroed local heap payload from one compiled allocation layout.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_layout(
        &mut self,
        layout: &HeapAllocationLayout<'_>,
    ) -> Result<HeapReference, Error> {
        unsafe { &mut *self.heap }
            .allocate_zeroed(layout)
            .map_err(Error::from)
    }

    /// Reserve one zeroed no-scan local heap allocation from the active young run.
    #[inline(always)]
    pub(crate) fn reserve_young(&mut self, small: SmallAllocationLayout) -> Option<HeapReference> {
        unsafe { &mut *self.heap }.reserve_young(small)
    }

    /// Allocate one byte-initialized local heap payload from one program layout id.
    #[inline(always)]
    pub(crate) fn allocate_heap_layout_bytes(
        &mut self,
        layout_id: mir::LayoutId,
        bytes: &[u8],
    ) -> Result<HeapReference, Error> {
        let shape = self.program.allocation_shape(layout_id)?;
        let heap = unsafe { &mut *self.heap };
        let layout = heap.allocation_layout(shape);

        heap.allocate_bytes(&layout, bytes).map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one allocation shape.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_shape(
        &mut self,
        shape: AllocationShape<'_>,
    ) -> Result<SharedHeapReference, Error> {
        let shared = unsafe { &*self.shared };
        let allocator = unsafe { &mut *self.shared_allocator };
        let layout = shared.allocation_layout(shape);

        shared
            .allocate_zeroed(self.shared_gc, allocator, &layout)
            .map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one compiled allocation layout.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_layout(
        &mut self,
        layout: &HeapAllocationLayout<'_>,
    ) -> Result<SharedHeapReference, Error> {
        let shared = unsafe { &*self.shared };
        let allocator = unsafe { &mut *self.shared_allocator };

        shared
            .allocate_zeroed(self.shared_gc, allocator, layout)
            .map_err(Error::from)
    }

    /// Reserve one zeroed no-scan shared heap allocation from the active worker run.
    #[inline(always)]
    pub(crate) fn reserve_shared_small(
        &mut self,
        small: SmallAllocationLayout,
    ) -> Option<SharedHeapReference> {
        let shared = unsafe { &*self.shared };
        if shared.gc_phase() != SharedGcPhase::Idle {
            return None;
        }

        unsafe { &mut *self.shared_allocator }.reserve_small_zeroed(small)
    }

    /// Borrow the worker heap immutably.
    #[inline]
    pub(crate) fn heap(&self) -> &Heap {
        unsafe { &*self.heap }
    }

    /// Borrow the shared heap immutably.
    #[inline]
    pub(crate) fn shared(&self) -> &SharedHeap {
        unsafe { &*self.shared }
    }

    /// Publish allocator-local shared heap runs.
    #[inline(always)]
    pub(crate) fn flush_shared_allocator(&mut self) {
        unsafe { &*self.shared }.flush_allocator(unsafe { &mut *self.shared_allocator });
    }

    /// Move the machine to a new frame and function.
    pub(crate) fn enter_frame(
        &mut self,
        frame_index: usize,
        function: &Function,
    ) -> Result<(), Error> {
        // validate frame index in debug builds
        debug_assert!(
            frame_index < self.interpreter.frames.len(),
            "frame index out of bounds"
        );

        // update cached frame pointer
        let frame = unsafe { self.interpreter.frames.get_unchecked_mut(frame_index) };
        self.frame_index = frame_index;
        self.frame = frame as *mut Frame;

        // refresh cached pointers
        self.refresh_frame(function)
    }

    /// Get the current frame mutably.
    #[inline(always)]
    pub(crate) fn current_frame_mut(&mut self) -> &mut Frame {
        unsafe { &mut *self.frame }
    }

    /// Borrow the current frame layout.
    #[inline(always)]
    pub(crate) fn frame_layout(&self) -> &engine::FrameLayout {
        unsafe { &*self.frame_layout }
    }

    /// Borrow one frame.
    #[inline(always)]
    pub(crate) fn frame(&self, frame_index: usize) -> Result<&Frame, Error> {
        self.interpreter
            .frames
            .get(frame_index)
            .ok_or(Error::InvalidInstruction)
    }

    /// Return whether the interpreter owns one stack byte range.
    #[inline]
    pub(crate) fn owns_stack_range(&self, pointer: StackPointer, byte_len: usize) -> bool {
        self.owns_stack_address_range(pointer.address(), byte_len)
    }

    /// Return whether the interpreter owns one frame byte range.
    #[inline]
    pub(crate) fn owns_frame_range(&self, pointer: FramePointer, byte_len: usize) -> bool {
        self.owns_stack_address_range(pointer.address(), byte_len)
    }

    /// Return whether the interpreter owns one stack address range.
    #[inline]
    fn owns_stack_address_range(&self, address: usize, byte_len: usize) -> bool {
        self.interpreter.stack.contains_address(address, byte_len)
    }

    /// Allocate bytes owned by the current frame.
    pub(crate) fn allocate_stack(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> Result<usize, Error> {
        let base = self
            .interpreter
            .stack
            .allocate(byte_len, alignment)
            .map_err(|_| Error::StackOverflow)?;
        let end = self.interpreter.stack.len();
        self.current_frame_mut().extend_bytes_to(end);

        Ok(self
            .interpreter
            .stack
            .address(base, byte_len)
            .map_err(|_| Error::StackOverflow)? as usize)
    }

    /// Retire the most recent stack allocation owned by the current frame.
    pub(crate) fn retire_stack(
        &mut self,
        pointer: StackPointer,
        byte_len: usize,
    ) -> Result<(), Error> {
        let offset = self
            .interpreter
            .stack
            .offset_for_address(pointer.address(), byte_len)
            .ok_or(Error::InvalidAddressSpace {
                expected: "stack".to_string(),
                actual: format!("0x{:x}", pointer.address()),
            })?;

        // stack allocations are bump allocated and must retire in reverse order
        let end = offset + byte_len;
        if end != self.interpreter.stack.len() {
            return Err(Error::InvalidAddressSpace {
                expected: "top of stack".to_string(),
                actual: format!("0x{:x}", pointer.address()),
            });
        }

        // the current frame owns all stack allocations made while it runs
        let frame = self.current_frame_mut();
        if offset < frame.stack_offset {
            return Err(Error::InvalidAddressSpace {
                expected: "current frame stack".to_string(),
                actual: format!("0x{:x}", pointer.address()),
            });
        }

        self.interpreter.truncate_stack(offset);
        let frame = self.current_frame_mut();
        frame.truncate_bytes_to(offset);

        Ok(())
    }

    /// Return whether one static pointer targets VM-owned static memory.
    #[inline]
    pub(crate) fn owns_static_range(&self, pointer: StaticPointer, byte_len: usize) -> bool {
        self.statics.owns_pointer_range(pointer, byte_len)
            || self.program.owns_static_range(pointer, byte_len)
    }

    /// Return whether one static pointer targets mutable worker static memory.
    #[inline]
    pub(crate) fn owns_mutable_static_range(
        &self,
        pointer: StaticPointer,
        byte_len: usize,
    ) -> bool {
        self.statics.owns_mutable_pointer_range(pointer, byte_len)
    }

    /// Return the static pointer for one global.
    #[inline]
    pub(crate) fn static_pointer(
        &self,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<StaticPointer> {
        self.statics
            .pointer(self.program.static_id(global))
            .or_else(|| self.program.static_pointer(global))
    }

    /// Get value by SSA id.
    #[inline(always)]
    pub(crate) fn get(&self, v: mir::Value) -> Word {
        let slot = self.value_slot_unchecked(v);

        unsafe { (*self.frame).read_slot_value(slot) }
    }

    /// Read one word by frame byte offset.
    #[inline(always)]
    pub(crate) fn get_word_at(&self, offset: u32) -> Word {
        unsafe { (*self.frame).read_word_at(offset) }
    }

    /// Return one frame pointer by frame byte offset.
    #[inline(always)]
    pub(crate) fn frame_pointer_at(&self, offset: u32) -> FramePointer {
        let address = unsafe { (*self.frame).base_address() } + offset as usize;

        FramePointer::from_address(address)
    }

    /// Return the frame slot for one SSA value without release checks.
    #[inline(always)]
    fn value_slot_unchecked(&self, v: mir::Value) -> &engine::FrameSlot {
        let index = v.0 as usize;
        let layout = self.frame_layout();
        debug_assert!(
            index < layout.values().len(),
            "ssa value out of bounds: {v:?}"
        );

        unsafe { layout.values().get_unchecked(index) }
    }

    /// Write one word by SSA id.
    #[inline(always)]
    pub(crate) fn set_word(&mut self, v: mir::Value, val: Word) {
        let index = v.0 as usize;
        let layout = self.frame_layout();
        debug_assert!(
            index < layout.values().len(),
            "ssa value out of bounds: {v:?}"
        );
        let slot = unsafe { layout.values().get_unchecked(index) };

        debug_assert!(slot.is_word, "attempted word write into frame bytes");
        unsafe { (*self.frame).write_word(slot, val) }
    }

    /// Write one word by frame byte offset.
    #[inline(always)]
    pub(crate) fn set_word_at(&mut self, offset: u32, val: Word) {
        unsafe { (*self.frame).write_word_at(offset, val) }
    }

    /// Move one local variable into a frame byte offset.
    #[inline(always)]
    pub(crate) fn move_local_to_offset(
        &mut self,
        local_index: u32,
        destination_offset: u32,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let local_slot = layout
            .locals()
            .get(local_index as usize)
            .ok_or(Error::InvalidInstruction)?;
        let local_offset = local_slot.offset;
        let byte_len = local_slot.byte_len;

        self.copy_frame_bytes(local_offset, destination_offset, byte_len as usize);

        Ok(())
    }

    /// Move one frame byte offset into a local variable.
    #[inline(always)]
    pub(crate) fn move_offset_to_local(
        &mut self,
        source_offset: u32,
        local_index: u32,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let local_slot = layout
            .locals()
            .get(local_index as usize)
            .ok_or(Error::InvalidInstruction)?;
        let local_offset = local_slot.offset;
        let byte_len = local_slot.byte_len;

        self.copy_frame_bytes(source_offset, local_offset, byte_len as usize);

        Ok(())
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
        let frame = self.current_frame_mut();

        // lower guarantees that both ranges are inside the frame layout
        unsafe {
            std::ptr::copy(
                frame.base_address().wrapping_add(source_offset) as *const u8,
                frame.base_address().wrapping_add(destination_offset) as *mut u8,
                byte_len,
            );
        }
    }

    /// Get the argument slice for the given range.
    #[inline(always)]
    pub(crate) fn argument_slice(&self, range: ArgumentRange) -> &[mir::Value] {
        let start = range.start as usize;
        let len = range.len as usize;
        debug_assert!(
            start + len <= self.argument_pool_len,
            "argument pool out of bounds for range"
        );
        unsafe { std::slice::from_raw_parts(self.argument_pool.add(start), len) }
    }
}
