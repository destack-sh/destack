use std::{fmt, mem, ptr};

use destack_heap::{
    AllocationShape, Heap, HeapReference, HeapResult, Payload, RawAllocationShape,
    SharedAllocationCache, SharedGcWorker, SharedHeapReference, repeated_layout,
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
use crate::{FramePointer, RawPointer, SharedHeap, SharedRawPointer, StaticPointer, Word};

/// Execution context for one active interpreter frame.
///
/// Frame byte addresses are cached for the hot dispatch path.
pub(crate) struct Machine<'ctx, 'iso> {
    /// Program being executed.
    pub(crate) program: &'iso Program,
    /// Immutable isolate options.
    pub(crate) options: &'iso IsolateOptions,
    /// Mutable static byte arena.
    pub(crate) statics: &'iso mut StaticSpace,
    /// The worker-local heap borrowed for this dispatch step.
    heap: &'iso mut Heap,
    /// The runtime-shared heap borrowed for this dispatch step.
    shared: &'iso SharedHeap,
    /// Shared collector worker for allocation assist.
    shared_gc: &'iso SharedGcWorker,
    /// The worker cache for shared heap allocations borrowed for this dispatch step.
    shared_cache: &'iso mut SharedAllocationCache,
    /// Interpreter owning the live stack.
    pub(crate) interpreter: &'ctx mut Interpreter,

    /// Index of the current frame in the stack.
    pub frame_index: usize,
    /// Native address of the active frame bytes.
    frame_base: usize,
    /// Active frame layout.
    frame_layout: &'iso engine::FrameLayout,
    /// Program side table.
    side_table: &'iso SideTable,
    /// Argument pool for the current function.
    argument_pool: &'iso [mir::Value],
}

impl fmt::Debug for Machine<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Machine")
            .field("frame_index", &self.frame_index)
            .field("argument_pool_len", &self.argument_pool.len())
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
        shared_cache: &'iso mut SharedAllocationCache,
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
            .ok_or(Error::InvalidInstruction)?;

        Ok(Self {
            program,
            options,
            statics,
            shared_gc,
            heap,
            shared,
            shared_cache,
            interpreter,
            frame_index,
            frame_base,
            frame_layout,
            side_table: &program.side_table,
            argument_pool: function.argument_pool.as_slice(),
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

    /// Load cached dispatch metadata from the active frame.
    pub(crate) fn load_active_frame(&mut self) -> Result<(), Error> {
        let (function, frame_base, frame_layout) = {
            let frame = self.active_frame();

            (frame.function(), frame.base_address(), frame.frame_layout())
        };
        self.frame_base = frame_base;

        let frame_layout = self
            .program
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::InvalidInstruction)?;
        self.frame_layout = frame_layout;

        let function = self
            .program
            .functions
            .function_by_id(function)
            .ok_or(Error::InvalidInstruction)?;

        // load argument pool
        self.argument_pool = function.argument_pool.as_slice();

        Ok(())
    }

    /// Borrow the program side table.
    #[inline(always)]
    fn side_table(&self) -> &'iso SideTable {
        self.side_table
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

    /// Allocate one zeroed local heap payload from one compiled allocation site.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_site(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let side_table = self.side_table();
        let allocation = side_table.allocation_site(id);
        let class = side_table.allocation_class(allocation.class);
        let site = allocation.heap_site(class);
        if let Some(reference) = self.heap.try_allocate_site_zeroed(site) {
            return Ok(reference);
        }

        let trace_map = self.program.trace_map(allocation.trace_map)?;

        self.heap
            .allocate_site_zeroed(site, trace_map)
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
        let trace_map = self.program.trace_map(element.trace_map)?;
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment, None, &trace_map);
        let heap = &mut *self.heap;
        let layout = heap.allocation_plan(shape);

        heap.allocate_zeroed(&layout).map_err(Error::from)
    }

    /// Allocate one byte-initialized local heap payload from one program layout id.
    #[inline(always)]
    pub(crate) fn allocate_heap_layout_bytes(
        &mut self,
        layout_id: mir::LayoutId,
        bytes: &[u8],
    ) -> Result<HeapReference, Error> {
        let shape = self.program.allocation_shape(layout_id)?;
        let heap = &mut *self.heap;
        let layout = heap.allocation_plan(shape);

        heap.allocate_bytes(&layout, bytes).map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one compiled allocation site.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_site(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<SharedHeapReference, Error> {
        let side_table = self.side_table();
        let allocation = side_table.allocation_site(id);
        let class = side_table.allocation_class(allocation.class);
        let site = allocation.heap_site(class);
        if let Some(reference) = self
            .shared
            .try_allocate_site_zeroed(self.shared_cache, site)
        {
            return Ok(reference);
        }

        let trace_map = self.program.trace_map(allocation.trace_map)?;

        self.shared
            .allocate_site_zeroed(
                self.shared_gc,
                self.shared_cache,
                site,
                trace_map,
                self.program.trace_table(),
            )
            .map_err(Error::from)
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
        let trace_map = self.program.trace_map(element.trace_map)?;
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment, None, &trace_map);
        let layout = self.shared.allocation_plan(shape);

        self.shared
            .allocate_zeroed(
                self.shared_gc,
                self.shared_cache,
                &layout,
                self.program.trace_table(),
            )
            .map_err(Error::from)
    }

    /// Publish worker-local shared heap runs.
    #[inline(always)]
    pub(crate) fn flush_shared_cache(&mut self) {
        self.shared.flush_allocation_cache(self.shared_cache);
    }

    /// Return one local heap native address.
    #[inline(always)]
    pub(crate) fn heap_address(&self, reference: HeapReference, byte_offset: usize) -> usize {
        self.heap.heap_base_address() + reference.offset() + byte_offset
    }

    /// Return one shared heap native address.
    #[inline(always)]
    pub(crate) fn shared_heap_address(
        &self,
        reference: SharedHeapReference,
        byte_offset: usize,
    ) -> usize {
        self.shared.heap_base_address() + reference.offset() + byte_offset
    }

    /// Return one local raw native address.
    #[inline(always)]
    pub(crate) fn raw_address(&self, pointer: RawPointer, byte_offset: usize) -> usize {
        self.heap.raw_base_address() + pointer.offset() + byte_offset
    }

    /// Return one shared raw native address.
    #[inline(always)]
    pub(crate) fn shared_raw_address(
        &self,
        pointer: SharedRawPointer,
        byte_offset: usize,
    ) -> usize {
        self.shared.raw_base_address() + pointer.offset() + byte_offset
    }

    /// Return whether one local heap reference is live.
    #[inline(always)]
    pub(crate) fn is_heap_live(&self, reference: HeapReference) -> bool {
        self.heap.is_heap_live(reference)
    }

    /// Return whether one shared heap reference is live.
    #[inline(always)]
    pub(crate) fn is_shared_heap_live(&self, reference: SharedHeapReference) -> bool {
        self.shared.is_heap_live(reference)
    }

    /// Allocate one zeroed local raw allocation.
    #[inline(always)]
    pub(crate) fn allocate_raw_zeroed(
        &mut self,
        shape: RawAllocationShape,
    ) -> HeapResult<RawPointer> {
        self.heap.allocate_raw(shape, Payload::Zeroed)
    }

    /// Allocate one zeroed shared raw allocation.
    #[inline(always)]
    pub(crate) fn allocate_shared_raw_zeroed(
        &self,
        shape: RawAllocationShape,
    ) -> HeapResult<SharedRawPointer> {
        self.shared.allocate_raw(shape, Payload::Zeroed)
    }

    /// Free one local raw allocation.
    #[inline(always)]
    pub(crate) fn free_raw(&mut self, pointer: RawPointer) -> HeapResult<()> {
        self.heap.free_raw(pointer)
    }

    /// Free one shared raw allocation.
    #[inline(always)]
    pub(crate) fn free_shared_raw(&self, pointer: SharedRawPointer) -> HeapResult<()> {
        self.shared.free_raw(pointer)
    }

    /// Free one local managed allocation.
    #[inline(always)]
    pub(crate) fn free_heap(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap.free(reference)
    }

    /// Free one shared managed allocation.
    #[inline(always)]
    pub(crate) fn free_shared_heap(&self, reference: SharedHeapReference) -> HeapResult<()> {
        self.shared.free(reference)
    }

    /// Pin one local managed allocation.
    #[inline(always)]
    pub(crate) fn pin_heap(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap.pin(reference)
    }

    /// Unpin one local managed allocation.
    #[inline(always)]
    pub(crate) fn unpin_heap(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap.unpin(reference)
    }

    /// Record one local heap write barrier.
    #[inline(always)]
    pub(crate) fn write_heap_barrier(
        &mut self,
        reference: HeapReference,
        offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.heap
            .write_barrier(reference, offset, byte_len, self.program.trace_table())
    }

    /// Record one shared heap write barrier.
    #[inline(always)]
    pub(crate) fn write_shared_heap_barrier(
        &self,
        reference: SharedHeapReference,
        offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.shared
            .write_barrier(reference, offset, byte_len, self.program.trace_table())
    }

    /// Read one local raw allocation byte range.
    #[inline(always)]
    pub(crate) fn read_raw_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        destination: &mut [u8],
    ) -> HeapResult<()> {
        self.heap.read_raw_bytes_into(pointer, start, destination)
    }

    /// Write one local raw allocation byte range.
    #[inline(always)]
    pub(crate) fn write_raw_bytes(
        &mut self,
        pointer: RawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap.write_raw_bytes(pointer, start, bytes)
    }

    /// Move the machine to another live frame.
    pub(crate) fn enter_frame(&mut self, frame_index: usize) -> Result<(), Error> {
        let _frame = self
            .interpreter
            .frames
            .get(frame_index)
            .ok_or(Error::InvalidInstruction)?;
        self.frame_index = frame_index;

        self.load_active_frame()
    }

    /// Borrow the active frame mutably.
    #[inline(always)]
    pub(crate) fn active_frame_mut(&mut self) -> &mut Frame {
        debug_assert!(self.frame_index < self.interpreter.frames.len());

        // SAFETY: Machine is only constructed with a live frame index and updates it on frame entry
        unsafe { self.interpreter.frames.get_unchecked_mut(self.frame_index) }
    }

    /// Borrow the active frame.
    #[inline(always)]
    pub(crate) fn active_frame(&self) -> &Frame {
        debug_assert!(self.frame_index < self.interpreter.frames.len());

        // SAFETY: Machine is only constructed with a live frame index and updates it on frame entry
        unsafe { self.interpreter.frames.get_unchecked(self.frame_index) }
    }

    /// Borrow the current frame layout.
    #[inline(always)]
    pub(crate) fn frame_layout(&self) -> &engine::FrameLayout {
        self.frame_layout
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
            .map_err(|_| Error::StackOverflow)?;

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

    /// Read one aligned word from the current frame.
    #[inline(always)]
    fn read_frame_word(&self, offset: u32) -> Word {
        let address = self.frame_base + offset as usize;
        debug_assert_eq!(address % mem::align_of::<Word>(), 0);

        // SAFETY: lowered word offsets are word-aligned and point inside the active frame
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

        // SAFETY: lower only emits SSA values present in the active frame layout
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

        // SAFETY: lowered word offsets are word-aligned and point inside the active frame
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
        let start = range.start as usize;
        let len = range.len as usize;
        let end = start + len;
        debug_assert!(
            end <= self.argument_pool.len(),
            "argument pool out of bounds for range"
        );

        &self.argument_pool[start..end]
    }
}
