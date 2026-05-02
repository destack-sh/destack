use std::fmt;

use destack_heap::{
    AllocationLayout as HeapAllocationLayout, AllocationPlan, Heap, HeapReference, SharedAllocator,
    SharedGcPhase, SharedGcWorker, SharedHeapReference, SmallAllocationLayout,
};
use engine::StaticSpace;
use {destack_engine as engine, destack_mir as mir};

use super::{Frame, Interpreter};
use crate::diagnostic::{Error, RuntimeError};
use crate::options::IsolateOptions;
use crate::program::{
    ArgumentRange, ElementAccess, ElementAccessId, FieldAccess, FieldAccessId, FrameAccess,
    FrameAccessId, Function, Layout, OperandTable, PointeeAccess, PointeeAccessId, Program,
    SliceElementAccess, SliceElementAccessId,
};
use crate::{FramePointer, SharedHeap, StackPointer, StaticPointer, Word};

/// Cached state for one interpreter dispatch.
pub(crate) struct DispatchState<'ctx, 'iso> {
    /// Program executed by this dispatch.
    pub(crate) program: &'iso Program,
    /// Immutable isolate options.
    pub(crate) options: &'iso IsolateOptions,
    /// Mutable static byte arena.
    pub(crate) statics: &'iso mut StaticSpace,
    /// Shared collector worker for allocation assist.
    shared_gc: &'iso SharedGcWorker,
    /// The worker-local heap.
    heap: *mut Heap,
    /// The world-shared heap.
    shared: *const SharedHeap,
    /// The worker cache for shared heap allocations.
    shared_allocator: *mut SharedAllocator,
    /// Interpreter engine state for this dispatch.
    pub(crate) engine: &'ctx mut Interpreter,

    /// Index of the current frame in the stack.
    pub frame_index: usize,
    /// Whether bounds checks are enabled for this dispatch.
    pub bounds_checks: bool,
    /// Whether null checks are enabled for this dispatch.
    pub null_checks: bool,
    /// Whether reference address-space checks are enabled for this dispatch.
    pub reference_kind_checks: bool,
    /// Whether reference mutability checks are enabled for this dispatch.
    pub reference_mutability_checks: bool,
    /// Pointer to the current frame.
    frame: *mut Frame,
    /// Pointer to the current frame layout.
    frame_layout: *const engine::FrameLayout,
    /// Pointer to the program operand table.
    operand_table: *const OperandTable,
    /// Argument pool for the current function.
    argument_pool: *const mir::Value,
    /// Argument pool length.
    argument_pool_len: usize,
}

impl fmt::Debug for DispatchState<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DispatchState")
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

impl<'ctx, 'iso> DispatchState<'ctx, 'iso> {
    /// Create dispatch state for the current frame.
    pub(crate) fn new(
        program: &'iso Program,
        options: &'iso IsolateOptions,
        statics: &'iso mut StaticSpace,
        heap: &'iso mut Heap,
        shared: &'iso SharedHeap,
        shared_gc: &'iso SharedGcWorker,
        shared_allocator: &'iso mut SharedAllocator,
        engine: &'ctx mut Interpreter,
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
            frame_index < engine.frames.len(),
            "frame index out of bounds"
        );
        // safety: frame_index always points at the current frame
        let frame = unsafe { engine.frames.get_unchecked_mut(frame_index) as *mut Frame };

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
            engine,
            frame_index,
            bounds_checks,
            null_checks,
            reference_kind_checks,
            reference_mutability_checks,
            frame,
            frame_layout,
            operand_table: &program.operand_table,
            argument_pool: function.argument_pool.as_ptr(),
            argument_pool_len: function.argument_pool.len(),
        })
    }

    /// Borrow the program MIR tree.
    #[inline]
    pub(crate) fn tree(&self) -> &mir::Tree {
        &self.program.tree
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
        let region = self
            .frame_layout()
            .value(value.0)
            .ok_or(Error::InvalidInstruction)?;

        Ok(self.program.type_for_id(region.ty))
    }

    /// Return the frame region for one SSA value.
    #[inline]
    pub(crate) fn value_region(&self, value: mir::Value) -> Result<&engine::FrameRegion, Error> {
        self.frame_layout()
            .value(value.0)
            .ok_or(Error::InvalidInstruction)
    }

    /// Return whether one SSA value is stored as one word.
    #[inline]
    pub(crate) fn value_is_word(&self, value: mir::Value) -> Result<bool, Error> {
        Ok(self.value_region(value)?.is_word)
    }

    /// Return the frame address for one SSA value.
    #[inline]
    pub(crate) fn value_address(&self, value: mir::Value) -> Result<FramePointer, Error> {
        let region = self.value_region(value)?;
        let address = unsafe { (*self.frame).region_address(region) };

        Ok(FramePointer::from_address(address))
    }

    /// Return one value as the operand expected by pointer-style access helpers.
    #[inline]
    pub(crate) fn value_operand(&self, value: mir::Value) -> Result<Word, Error> {
        let region = self.value_region(value)?;
        if region.is_word {
            return Ok(self.get(value));
        }

        Ok(Word::frame_pointer(self.value_address(value)?))
    }

    /// Borrow one SSA value's bytes.
    #[inline]
    pub(crate) fn value_bytes(&self, value: mir::Value) -> Result<&[u8], Error> {
        let region = self.value_region(value)?;
        let frame = unsafe { &*self.frame };

        Ok(frame.region_bytes(region))
    }

    /// Return one SSA value's frame byte range.
    #[inline]
    pub(crate) fn frame_value_byte_range(
        &self,
        value: mir::Value,
    ) -> Result<(*const u8, usize), Error> {
        let region = self.value_region(value)?;
        let frame = unsafe { &*self.frame };
        let bytes = frame.region_bytes(region);

        Ok((bytes.as_ptr(), bytes.len()))
    }

    /// Borrow one SSA value's bytes mutably.
    #[inline]
    pub(crate) fn value_bytes_mut(&mut self, value: mir::Value) -> Result<&mut [u8], Error> {
        let region = self.value_region(value)? as *const engine::FrameRegion;
        let frame = self.current_frame_mut();

        Ok(frame.region_bytes_mut(unsafe { &*region }))
    }

    /// Borrow the isolate options.
    #[inline]
    pub(crate) fn options(&self) -> &IsolateOptions {
        self.options
    }

    /// Create a runtime error with current call stack.
    #[cold]
    pub(crate) fn runtime_error(&self, error: Error) -> RuntimeError {
        self.engine.runtime_error(self.program, error)
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

    /// Return the program operand table pointer.
    #[inline(always)]
    pub(crate) fn operand_table_ptr(&self) -> *const OperandTable {
        self.operand_table
    }

    /// Return one pooled field access.
    #[inline(always)]
    pub(crate) fn field_access(&self, id: FieldAccessId) -> FieldAccess {
        *unsafe { (*self.operand_table).field_access(id) }
    }

    /// Return one pooled frame access.
    #[inline(always)]
    pub(crate) fn frame_access(&self, id: FrameAccessId) -> FrameAccess {
        *unsafe { (*self.operand_table).frame_access(id) }
    }

    /// Return one pooled element access.
    #[inline(always)]
    pub(crate) fn element_access(&self, id: ElementAccessId) -> ElementAccess {
        *unsafe { (*self.operand_table).element_access(id) }
    }

    /// Return one pooled slice element access.
    #[inline(always)]
    pub(crate) fn slice_element_access(&self, id: SliceElementAccessId) -> SliceElementAccess {
        *unsafe { (*self.operand_table).slice_element_access(id) }
    }

    /// Return one pooled pointee access.
    #[inline(always)]
    pub(crate) fn pointee_access(&self, id: PointeeAccessId) -> PointeeAccess {
        *unsafe { (*self.operand_table).pointee_access(id) }
    }

    /// Borrow the heap for the current block.
    #[inline]
    pub(crate) fn heap_mut(&mut self) -> &mut Heap {
        unsafe { &mut *self.heap }
    }

    /// Allocate one zeroed local heap payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_plan(
        &mut self,
        plan: AllocationPlan<'_>,
    ) -> Result<HeapReference, Error> {
        let heap = unsafe { &mut *self.heap };
        let layout = heap.allocation_layout(plan);

        heap.allocate_zeroed(&layout).map_err(Error::from)
    }

    /// Allocate one zeroed local heap payload from one resolved allocation layout.
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
        let plan = self.program.allocation_plan(layout_id)?;
        let heap = unsafe { &mut *self.heap };
        let layout = heap.allocation_layout(plan);

        heap.allocate_bytes(&layout, bytes).map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_plan(
        &mut self,
        plan: AllocationPlan<'_>,
    ) -> Result<SharedHeapReference, Error> {
        let shared = unsafe { &*self.shared };
        let allocator = unsafe { &mut *self.shared_allocator };
        let layout = shared.allocation_layout(plan);

        shared
            .allocate_zeroed(self.shared_gc, allocator, &layout)
            .map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one resolved allocation layout.
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

    /// Borrow the heap immutably for the current block.
    #[inline]
    pub(crate) fn heap(&self) -> &Heap {
        unsafe { &*self.heap }
    }

    /// Borrow the shared heap immutably for the current block.
    #[inline]
    pub(crate) fn shared(&self) -> &SharedHeap {
        unsafe { &*self.shared }
    }

    /// Publish allocator-local shared heap runs.
    #[inline(always)]
    pub(crate) fn flush_shared_allocator(&mut self) {
        unsafe { &*self.shared }.flush_allocator(unsafe { &mut *self.shared_allocator });
    }

    /// Move the state to a new frame and function.
    pub(crate) fn enter_frame(
        &mut self,
        frame_index: usize,
        function: &Function,
    ) -> Result<(), Error> {
        // validate frame index in debug builds
        debug_assert!(
            frame_index < self.engine.frames.len(),
            "frame index out of bounds"
        );

        // update cached frame pointer
        let frame = unsafe { self.engine.frames.get_unchecked_mut(frame_index) };
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
        self.engine
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
        self.engine.stack.contains_address(address, byte_len)
    }

    /// Allocate bytes owned by the current frame.
    pub(crate) fn allocate_stack(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> Result<usize, Error> {
        let base = self
            .engine
            .stack
            .allocate(byte_len, alignment)
            .map_err(|_| Error::StackOverflow)?;
        let end = self.engine.stack.len();
        self.current_frame_mut().extend_bytes_to(end);

        Ok(self
            .engine
            .stack
            .address(base, byte_len)
            .map_err(|_| Error::StackOverflow)? as usize)
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

    /// Borrow static bytes for one global.
    #[inline]
    pub(crate) fn static_bytes(&self, global: mir::LocalNodeId<mir::Global>) -> Option<&[u8]> {
        self.statics
            .bytes(self.program.static_id(global))
            .or_else(|| self.program.static_bytes(global))
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
        let region = self.value_region_unchecked(v);

        unsafe { (*self.frame).read_operand(region) }
    }

    /// Read one word by SSA id.
    #[inline(always)]
    pub(crate) fn get_word(&self, v: mir::Value) -> Word {
        let region = self.value_region_unchecked(v);

        debug_assert!(region.is_word, "attempted word read from frame bytes");
        unsafe { (*self.frame).read_word(region) }
    }

    /// Read one word by frame byte offset.
    #[inline(always)]
    pub(crate) fn get_word_at(&self, offset: u32) -> Word {
        unsafe { (*self.frame).read_word_at(offset) }
    }

    /// Return the frame region for one SSA value without release checks.
    #[inline(always)]
    fn value_region_unchecked(&self, v: mir::Value) -> &engine::FrameRegion {
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
        let region = unsafe { layout.values().get_unchecked(index) };

        debug_assert!(region.is_word, "attempted word write into frame bytes");
        unsafe { (*self.frame).write_word(region, val) }
    }

    /// Write one word by frame byte offset.
    #[inline(always)]
    pub(crate) fn set_word_at(&mut self, offset: u32, val: Word) {
        unsafe { (*self.frame).write_word_at(offset, val) }
    }

    /// Move one local variable into one SSA value.
    #[inline(always)]
    pub(crate) fn move_local_to_value(
        &mut self,
        local_index: u32,
        value: mir::Value,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let local_region = layout
            .locals()
            .get(local_index as usize)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;
        let value_region =
            layout.value(value.0).ok_or(Error::InvalidInstruction)? as *const engine::FrameRegion;

        self.move_frame_region(unsafe { &*local_region }, unsafe { &*value_region })
    }

    /// Move one SSA value into one local variable.
    #[inline(always)]
    pub(crate) fn move_value_to_local(
        &mut self,
        value: mir::Value,
        local_index: u32,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let value_region =
            layout.value(value.0).ok_or(Error::InvalidInstruction)? as *const engine::FrameRegion;
        let local_region = layout
            .locals()
            .get(local_index as usize)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;

        self.move_frame_region(unsafe { &*value_region }, unsafe { &*local_region })
    }

    /// Move one SSA value into another SSA value.
    #[inline(always)]
    pub(crate) fn move_value_to_value(
        &mut self,
        source: mir::Value,
        destination: mir::Value,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let source_region =
            layout.value(source.0).ok_or(Error::InvalidInstruction)? as *const engine::FrameRegion;
        let destination_region = layout
            .value(destination.0)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;

        self.move_frame_region(unsafe { &*source_region }, unsafe { &*destination_region })
    }

    /// Move one frame byte range into another frame byte range.
    #[inline(always)]
    pub(crate) fn move_value_range(
        &mut self,
        destination: mir::Value,
        destination_offset: usize,
        source: mir::Value,
        source_offset: usize,
        byte_len: usize,
    ) -> Result<(), Error> {
        let (source, source_len) = self.frame_value_byte_range(source)?;
        let destination = self.value_bytes_mut(destination)?;

        let source_end = source_offset + byte_len;
        let destination_end = destination_offset + byte_len;
        if source_end > source_len || destination_end > destination.len() {
            return Err(Error::InvalidInstruction);
        }

        unsafe {
            std::ptr::copy(
                source.add(source_offset),
                destination.as_mut_ptr().add(destination_offset),
                byte_len,
            );
        }

        Ok(())
    }

    /// Move bytes between two frame regions in the current frame.
    #[inline(always)]
    fn move_frame_region(
        &mut self,
        source: &engine::FrameRegion,
        destination: &engine::FrameRegion,
    ) -> Result<(), Error> {
        if source.byte_len != destination.byte_len || source.is_word != destination.is_word {
            return Err(Error::TypeMismatch {
                expected: format!(
                    "{} bytes, word={}",
                    destination.byte_len, destination.is_word
                ),
                actual: format!("{} bytes, word={}", source.byte_len, source.is_word),
            });
        }

        let frame = self.current_frame_mut();
        if source.is_word {
            let value = frame.read_word(source);
            frame.write_word(destination, value);

            return Ok(());
        }

        unsafe {
            std::ptr::copy(
                frame.region_address(source) as *const u8,
                frame.region_address(destination) as *mut u8,
                source.byte_len as usize,
            );
        }

        Ok(())
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
