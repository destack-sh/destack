use std::{fmt, mem, ptr};

use destack_heap::{
    AllocationShape, Heap, HeapReference, SharedAllocator, SharedGcWorker, SharedHeapReference,
    repeated_layout,
};
use engine::StaticSpace;
use {destack_engine as engine, destack_mir as mir};

use super::{Frame, Interpreter};
use crate::diagnostic::{Error, RuntimeError};
use crate::options::IsolateOptions;
use crate::program::{
    AllocationSiteId, ArgumentRange, Check, CheckId, Edge, EdgeId, Function, Instruction, Layout,
    Program, Projection, ProjectionId, SideRecord, SideTable, SliceProjection, SliceProjectionId,
    SwitchCasesId, SwitchTable, SwitchTableId, TensorConvolutionId, TensorDotId, TensorGatherId,
    TensorLayout, TensorLayoutId, TensorScatterId, TensorWindowId, U32RangeId,
};
use crate::{FramePointer, SharedHeap, StaticPointer, Word};

/// Execution context for one active interpreter frame.
///
/// Raw pointers cache borrows that are proven by the interpreter dispatch loop.
/// This keeps the hot path address-based while the caller owns the real lifetimes.
pub(crate) struct Machine<'ctx, 'iso> {
    /// Program being executed.
    pub(crate) program: &'iso Program,
    /// Immutable isolate options.
    pub(crate) options: &'iso IsolateOptions,
    /// Mutable static byte arena.
    pub(crate) statics: &'iso mut StaticSpace,
    /// The worker-local heap borrowed for this dispatch step.
    heap: *mut Heap,
    /// The world-shared heap borrowed for this dispatch step.
    shared: *const SharedHeap,
    /// Shared collector worker for allocation assist.
    shared_gc: &'iso SharedGcWorker,
    /// The worker cache for shared heap allocations borrowed for this dispatch step.
    shared_allocator: *mut SharedAllocator,
    /// Interpreter owning the live stack.
    pub(crate) interpreter: &'ctx mut Interpreter,

    /// Index of the current frame in the stack.
    pub frame_index: usize,
    /// Pointer to the active frame.
    frame: *mut Frame,
    /// Native address of the active frame bytes.
    frame_base: usize,
    /// Pointer to the active frame layout.
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
        let frame = interpreter
            .frames
            .get_mut(frame_index)
            .ok_or(Error::InvalidInstruction)?;
        let frame_base = frame.base_address();
        let frame_layout = frame.frame_layout();
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameLayout;
        let frame = frame as *mut Frame;

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
            frame,
            frame_base,
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
        T::get(self.side_table(), instruction.a)
    }

    /// Borrow one pooled side record by id.
    #[inline(always)]
    pub(crate) fn side_record<T: SideRecord>(&self, id: u32) -> &'iso T {
        T::get(self.side_table(), id)
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

        Ok(self.program.type_for_value_layout(slot.layout))
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
        let (frame_base, frame_layout) = {
            let frame = self.active_frame();

            (frame.base_address(), frame.frame_layout())
        };
        self.frame_base = frame_base;

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

    /// Borrow the program side table.
    #[inline(always)]
    fn side_table(&self) -> &'iso SideTable {
        unsafe { &*self.side_table }
    }

    /// Return one pooled address projection.
    #[inline(always)]
    pub(crate) fn projection(&self, id: ProjectionId) -> Projection {
        *self.side_table().projection(id)
    }

    /// Return one pooled slice projection.
    #[inline(always)]
    pub(crate) fn slice_projection(&self, id: SliceProjectionId) -> SliceProjection {
        *self.side_table().slice_projection(id)
    }

    /// Borrow one pooled check constraint.
    #[inline(always)]
    pub(crate) fn check(&self, id: CheckId) -> &'iso Check {
        self.side_table().check(id)
    }

    /// Return one pooled control edge.
    #[inline(always)]
    pub(crate) fn edge(&self, id: EdgeId) -> Edge {
        self.side_table().edge(id)
    }

    /// Borrow one pooled switch case table.
    #[inline(always)]
    pub(crate) fn switch_cases(&self, id: SwitchCasesId) -> &'iso [crate::program::SwitchCase] {
        self.side_table().switch_cases(id)
    }

    /// Borrow one pooled dense switch table.
    #[inline(always)]
    pub(crate) fn switch_table(&self, id: SwitchTableId) -> &'iso SwitchTable {
        self.side_table().switch_table(id)
    }

    /// Borrow one pooled u32 range.
    #[inline(always)]
    pub(crate) fn u32_range(&self, id: U32RangeId) -> &'iso [u32] {
        self.side_table().u32_range(id)
    }

    /// Borrow one pooled tensor layout.
    #[inline(always)]
    pub(crate) fn tensor_layout(&self, id: TensorLayoutId) -> &'iso TensorLayout {
        self.side_table().tensor_layout(id)
    }

    /// Borrow one pooled tensor dot descriptor.
    #[inline(always)]
    pub(crate) fn tensor_dot(&self, id: TensorDotId) -> &'iso mir::TensorDotDimensionNumbers {
        self.side_table().tensor_dot(id)
    }

    /// Borrow one pooled tensor convolution dimension descriptor.
    #[inline(always)]
    pub(crate) fn tensor_convolution(
        &self,
        id: TensorConvolutionId,
    ) -> &'iso mir::TensorConvolutionDimensionNumbers {
        self.side_table().tensor_convolution(id)
    }

    /// Borrow one pooled tensor convolution window descriptor.
    #[inline(always)]
    pub(crate) fn tensor_window(&self, id: TensorWindowId) -> &'iso mir::TensorConvolutionWindow {
        self.side_table().tensor_window(id)
    }

    /// Borrow one pooled tensor gather descriptor.
    #[inline(always)]
    pub(crate) fn tensor_gather(
        &self,
        id: TensorGatherId,
    ) -> &'iso mir::TensorGatherDimensionNumbers {
        self.side_table().tensor_gather(id)
    }

    /// Borrow one pooled tensor scatter descriptor.
    #[inline(always)]
    pub(crate) fn tensor_scatter(
        &self,
        id: TensorScatterId,
    ) -> &'iso mir::TensorScatterDimensionNumbers {
        self.side_table().tensor_scatter(id)
    }

    /// Borrow the worker heap mutably.
    #[inline]
    pub(crate) fn heap_mut(&mut self) -> &mut Heap {
        unsafe { &mut *self.heap }
    }

    /// Borrow the shared allocation cache mutably.
    #[inline(always)]
    fn shared_allocator_mut(&mut self) -> &mut SharedAllocator {
        unsafe { &mut *self.shared_allocator }
    }

    /// Allocate one zeroed local heap payload from one pooled allocation site.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_allocation(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let side_table = self.side_table();
        let allocation = side_table.allocation_site(id);
        let trace_map = side_table.trace_map(allocation.trace_map);
        let class = side_table.allocation_class(allocation.class);
        let layout = allocation.heap_plan(trace_map, class);

        self.heap_mut()
            .allocate_zeroed(&layout)
            .map_err(Error::from)
    }

    /// Allocate one zeroed local slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_slice(
        &mut self,
        element: AllocationSiteId,
        length: usize,
    ) -> Result<HeapReference, Error> {
        let side_table = self.side_table();
        let element = side_table.allocation_site(element);
        let trace_map = side_table.trace_map(element.trace_map);
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment, &trace_map);
        let heap = self.heap_mut();
        let layout = heap.allocation_plan(shape);

        heap.allocate_zeroed(&layout).map_err(Error::from)
    }

    /// Reserve one zeroed no-scan local heap allocation from the active young run.
    #[inline(always)]
    pub(crate) fn reserve_young(&mut self, slot_bytes: usize) -> Option<HeapReference> {
        self.heap_mut().reserve_young(slot_bytes)
    }

    /// Allocate one byte-initialized local heap payload from one program layout id.
    #[inline(always)]
    pub(crate) fn allocate_heap_layout_bytes(
        &mut self,
        layout_id: mir::LayoutId,
        bytes: &[u8],
    ) -> Result<HeapReference, Error> {
        let shape = self.program.allocation_shape(layout_id)?;
        let heap = self.heap_mut();
        let layout = heap.allocation_plan(shape);

        heap.allocate_bytes(&layout, bytes).map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one pooled allocation site.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_allocation(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<SharedHeapReference, Error> {
        let side_table = self.side_table();
        let allocation = side_table.allocation_site(id);
        let trace_map = side_table.trace_map(allocation.trace_map);
        let class = side_table.allocation_class(allocation.class);
        let layout = allocation.heap_plan(trace_map, class);

        unsafe {
            (&*self.shared)
                .allocate_zeroed(self.shared_gc, &mut *self.shared_allocator, &layout)
                .map_err(Error::from)
        }
    }

    /// Allocate one zeroed shared slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_slice(
        &mut self,
        element: AllocationSiteId,
        length: usize,
    ) -> Result<SharedHeapReference, Error> {
        let side_table = self.side_table();
        let element = side_table.allocation_site(element);
        let trace_map = side_table.trace_map(element.trace_map);
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment, &trace_map);
        let layout = self.shared().allocation_plan(shape);

        unsafe {
            (&*self.shared)
                .allocate_zeroed(self.shared_gc, &mut *self.shared_allocator, &layout)
                .map_err(Error::from)
        }
    }

    /// Reserve one zeroed no-scan shared heap allocation from the active worker run.
    #[inline(always)]
    pub(crate) fn reserve_shared_small(
        &mut self,
        bucket_index: usize,
        slot_bytes: usize,
    ) -> Option<SharedHeapReference> {
        unsafe {
            self.shared_allocator_mut()
                .reserve_zeroed_run_slot_unchecked(bucket_index, slot_bytes)
        }
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
        let frame = self
            .interpreter
            .frames
            .get_mut(frame_index)
            .ok_or(Error::InvalidInstruction)?;
        self.frame_index = frame_index;
        self.frame = frame as *mut Frame;

        // refresh cached pointers
        self.refresh_frame(function)
    }

    /// Borrow the active frame mutably.
    #[inline(always)]
    pub(crate) fn active_frame_mut(&mut self) -> &mut Frame {
        unsafe { &mut *self.frame }
    }

    /// Borrow the active frame.
    #[inline(always)]
    pub(crate) fn active_frame(&self) -> &Frame {
        unsafe { &*self.frame }
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
        self.active_frame_mut().extend_bytes_to(end);
        let address = self
            .interpreter
            .stack
            .address(base, byte_len)
            .map_err(|_| Error::StackOverflow)? as usize;

        Ok(address)
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

    /// Load one SSA value as a VM word.
    #[inline(always)]
    pub(crate) fn load_value(&self, v: mir::Value) -> Word {
        let slot = self.value_slot_unchecked(v);

        // word values live inline in the frame
        if slot.is_word {
            return self.read_frame_word(slot.offset);
        }

        // aggregate SSA values are represented by their frame address
        Word::frame_pointer(FramePointer::from_address(
            self.frame_base + slot.offset as usize,
        ))
    }

    /// Read one word by frame byte offset.
    #[inline(always)]
    pub(crate) fn load_word_at(&self, offset: u32) -> Word {
        self.read_frame_word(offset)
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

        // lower guarantees the frame range is valid for the operation
        unsafe {
            let bytes = std::slice::from_raw_parts(address as *const u8, byte_len);

            operation(self, bytes)
        }
    }

    /// Store frame bytes at one byte offset.
    #[inline(always)]
    pub(crate) fn store_frame_bytes_at(&mut self, offset: u32, bytes: &[u8]) {
        let address = self.frame_base + offset as usize;

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
        }
    }

    /// Copy frame bytes to one native address.
    #[inline(always)]
    pub(crate) fn copy_frame_bytes_at(&self, source: u32, destination: usize, byte_len: usize) {
        let source = self.frame_base + source as usize;

        unsafe {
            ptr::copy(source as *const u8, destination as *mut u8, byte_len);
        }
    }

    /// Read one aligned word from the current frame.
    #[inline(always)]
    fn read_frame_word(&self, offset: u32) -> Word {
        let address = self.frame_base + offset as usize;
        debug_assert_eq!(address % mem::align_of::<Word>(), 0);

        unsafe { ptr::read(address as *const Word) }
    }

    /// Return the frame slot for one SSA value without bounds checks.
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

    /// Store one word into an SSA value.
    #[inline(always)]
    pub(crate) fn store_value_word(&mut self, v: mir::Value, val: Word) {
        let slot = self.value_slot_unchecked(v);
        let is_word = slot.is_word;
        let offset = slot.offset;

        debug_assert!(is_word, "attempted word write into frame bytes");
        self.write_frame_word(offset, val);
    }

    /// Write one word by frame byte offset.
    #[inline(always)]
    pub(crate) fn store_word_at(&mut self, offset: u32, val: Word) {
        self.write_frame_word(offset, val);
    }

    /// Write one aligned word into the current frame.
    #[inline(always)]
    fn write_frame_word(&mut self, offset: u32, value: Word) {
        let address = self.frame_base + offset as usize;
        debug_assert_eq!(address % mem::align_of::<Word>(), 0);

        unsafe {
            ptr::write(address as *mut Word, value);
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

        // lower guarantees that both ranges are inside the frame layout
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
        let start = range.start as usize;
        let len = range.len as usize;
        debug_assert!(
            start + len <= self.argument_pool_len,
            "argument pool out of bounds for range"
        );
        unsafe { std::slice::from_raw_parts(self.argument_pool.add(start), len) }
    }
}
