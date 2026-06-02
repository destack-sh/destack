use std::{mem, ptr};

use destack_engine as engine;
use destack_engine::StaticAddress;
use destack_heap::{
    AllocationShape, AllocationSite as HeapAllocationSite, HeapReference, HeapResult, SharedHeap,
    SharedHeapReference, SmallAllocationPlan, SmallAllocationSite as HeapSmallAllocationSite,
    repeated_layout,
};
use destack_mir::{self as mir, TraceId};

use super::{Activation, Frame};
use crate::diagnostic::Error;
use crate::program::{
    AllocationSiteId, ArgumentRange, Check, CheckId, Edge, EdgeId, Instruction, Layout, Projection,
    ProjectionId, SideRecord, SideTable, SliceProjection, SliceProjectionId, SmallAllocationSiteId,
    SwitchCase, SwitchCasesId, SwitchTable, SwitchTableId, TensorConvolutionId, TensorDotId,
    TensorGatherId, TensorLayout, TensorLayoutId, TensorScatterId, TensorWindowId, U32RangeId,
};
use crate::{Cell, FramePointer};

impl Activation<'_> {
    /// Borrow one pooled side record.
    #[inline(always)]
    pub(crate) fn side<T: SideRecord + 'static>(&self, instruction: &Instruction) -> &'static T {
        let record = T::get(self.side_table(), instruction.a);

        // SAFETY: side records live in the machine program for the duration of dispatch
        unsafe { &*(record as *const T) }
    }

    /// Borrow one pooled side record by id.
    #[inline(always)]
    pub(crate) fn side_record<T: SideRecord + 'static>(&self, id: u32) -> &'static T {
        let record = T::get(self.side_table(), id);

        // SAFETY: side records live in the machine program for the duration of dispatch
        unsafe { &*(record as *const T) }
    }

    /// Return the compiled layout for one MIR type.
    #[inline]
    pub(crate) fn require_layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&Layout, Error> {
        self.machine
            .program
            .layout(ty)
            .ok_or_else(|| Error::type_mismatch("compiled layout", format!("{ty:?}")))
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
            .ok_or(Error::invalid_instruction())?;

        Ok(self.machine.program.type_for_storage_id(slot.layout))
    }

    /// Return the frame slot for one SSA value.
    #[inline]
    pub(crate) fn value_slot(&self, value: mir::Value) -> Result<&engine::FrameSlot, Error> {
        self.frame_layout()
            .value(value.0)
            .ok_or(Error::invalid_instruction())
    }

    /// Return whether one SSA value is stored as one cell.
    #[inline]
    pub(crate) fn value_is_cell(&self, value: mir::Value) -> Result<bool, Error> {
        Ok(self.value_slot(value)?.is_cell)
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

    /// Clear the last fallible allocation failure.
    #[inline]
    pub(crate) fn clear_allocation_failure(&mut self) {
        self.machine.last_allocation_failure = None;
    }

    /// Record one fallible allocation failure.
    #[inline]
    pub(crate) fn set_allocation_failure(&mut self, error: Error) {
        self.machine.last_allocation_failure = Some(error);
    }

    /// Borrow the program side table.
    #[inline(always)]
    fn side_table(&self) -> &'static SideTable {
        let side_table = &self.machine.program.side_table;

        // SAFETY: side tables live in the machine program for the duration of dispatch
        unsafe { &*(side_table as *const SideTable) }
    }

    /// Borrow static memory for this activation.
    #[inline(always)]
    pub(crate) fn statics(&self) -> &engine::StaticSpace {
        self.statics
    }

    /// Borrow the worker heap for this activation.
    #[inline(always)]
    fn heap(&self) -> &destack_heap::Heap {
        self.heap
    }

    /// Borrow the worker heap mutably for this activation.
    #[inline(always)]
    fn heap_mut(&mut self) -> &mut destack_heap::Heap {
        self.heap
    }

    /// Borrow the shared heap for this activation.
    #[inline(always)]
    fn shared(&self) -> &SharedHeap {
        self.shared
    }

    /// Borrow the shared allocation cache for this activation.
    #[inline(always)]
    fn shared_cache(&self) -> &destack_heap::AllocationCache {
        self.shared_cache
    }

    /// Reserve one small shared payload from this activation.
    #[inline(always)]
    fn reserve_shared_small(&mut self, plan: SmallAllocationPlan) -> Option<SharedHeapReference> {
        self.shared
            .reserve_small_from_cache(self.shared_cache, plan)
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
    pub(crate) fn check(&self, id: CheckId) -> &'static Check {
        self.side_table().check(id)
    }

    /// Return one pooled control edge.
    #[inline(always)]
    pub(crate) fn edge(&self, id: EdgeId) -> Edge {
        self.side_table().edge(id)
    }

    /// Borrow one pooled switch case table.
    #[inline(always)]
    pub(crate) fn switch_cases(&self, id: SwitchCasesId) -> &'static [SwitchCase] {
        self.side_table().switch_cases(id)
    }

    /// Borrow one pooled dense switch table.
    #[inline(always)]
    pub(crate) fn switch_table(&self, id: SwitchTableId) -> &'static SwitchTable {
        self.side_table().switch_table(id)
    }

    /// Borrow one pooled u32 range.
    #[inline(always)]
    pub(crate) fn u32_range(&self, id: U32RangeId) -> &'static [u32] {
        self.side_table().u32_range(id)
    }

    /// Borrow one pooled tensor layout.
    #[inline(always)]
    pub(crate) fn tensor_layout(&self, id: TensorLayoutId) -> &'static TensorLayout {
        self.side_table().tensor_layout(id)
    }

    /// Borrow one pooled tensor dot descriptor.
    #[inline(always)]
    pub(crate) fn tensor_dot(&self, id: TensorDotId) -> &'static mir::TensorDotDimensionNumbers {
        self.side_table().tensor_dot(id)
    }

    /// Borrow one pooled tensor convolution dimension descriptor.
    #[inline(always)]
    pub(crate) fn tensor_convolution(
        &self,
        id: TensorConvolutionId,
    ) -> &'static mir::TensorConvolutionDimensionNumbers {
        self.side_table().tensor_convolution(id)
    }

    /// Borrow one pooled tensor convolution window descriptor.
    #[inline(always)]
    pub(crate) fn tensor_window(
        &self,
        id: TensorWindowId,
    ) -> &'static mir::TensorConvolutionWindow {
        self.side_table().tensor_window(id)
    }

    /// Borrow one pooled tensor gather descriptor.
    #[inline(always)]
    pub(crate) fn tensor_gather(
        &self,
        id: TensorGatherId,
    ) -> &'static mir::TensorGatherDimensionNumbers {
        self.side_table().tensor_gather(id)
    }

    /// Borrow one pooled tensor scatter descriptor.
    #[inline(always)]
    pub(crate) fn tensor_scatter(
        &self,
        id: TensorScatterId,
    ) -> &'static mir::TensorScatterDimensionNumbers {
        self.side_table().tensor_scatter(id)
    }

    /// Return one trace map id and heap allocation site.
    #[inline(always)]
    fn heap_allocation_site(&self, id: AllocationSiteId) -> (TraceId, HeapAllocationSite) {
        let side_table = self.side_table();
        let vm_site = *side_table.allocation_site(id);

        (vm_site.trace_map, vm_site.heap)
    }

    /// Return one trace map id, cold heap site, and small heap site.
    #[inline(always)]
    fn heap_small_allocation_site(
        &self,
        id: SmallAllocationSiteId,
    ) -> (TraceId, HeapAllocationSite, HeapSmallAllocationSite) {
        let side_table = self.side_table();
        let vm_site = *side_table.small_allocation_site(id);

        (vm_site.trace_map, vm_site.heap, vm_site.small)
    }

    /// Allocate one zeroed local heap payload from one allocation site.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site) = self.heap_allocation_site(id);
        self.allocate_zeroed_heap_shape(trace_map, heap_site)
    }

    /// Allocate one zeroed local noscan small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_small_noscan(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.heap_mut().reserve_small_noscan(small_site) {
            return Ok(reference);
        }

        self.allocate_zeroed_heap_shape(trace_map, heap_site)
    }

    /// Allocate one zeroed local scanned small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_small_scan(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.heap_mut().reserve_small_scan(small_site) {
            return Ok(reference);
        }

        self.allocate_zeroed_heap_shape(trace_map, heap_site)
    }

    /// Allocate one zeroed local small heap payload that may point into shared heap.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_small_shared_edge(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.heap_mut().reserve_small_shared_edge(small_site) {
            return Ok(reference);
        }

        self.allocate_zeroed_heap_shape(trace_map, heap_site)
    }

    /// Allocate one zeroed local heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_zeroed_heap_shape(
        &mut self,
        trace_map: TraceId,
        heap_site: HeapAllocationSite,
    ) -> Result<HeapReference, Error> {
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(trace_map)?;

        self.heap_mut()
            .allocate_zeroed(heap_site, trace_map)
            .map_err(Error::from)
    }

    /// Allocate one uninitialized local heap payload from one allocation site.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site) = self.heap_allocation_site(id);
        self.allocate_uninit_heap_shape(trace_map, heap_site)
    }

    /// Allocate one uninitialized local noscan small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_small_noscan(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.heap_mut().reserve_small_noscan(small_site) {
            return Ok(reference);
        }

        self.allocate_uninit_heap_shape(trace_map, heap_site)
    }

    /// Allocate one uninitialized local scanned small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_small_scan(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.heap_mut().reserve_small_scan(small_site) {
            return Ok(reference);
        }

        self.allocate_uninit_heap_shape(trace_map, heap_site)
    }

    /// Allocate one uninitialized local small heap payload that may point into shared heap.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_small_shared_edge(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<HeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.heap_mut().reserve_small_shared_edge(small_site) {
            return Ok(reference);
        }

        self.allocate_uninit_heap_shape(trace_map, heap_site)
    }

    /// Allocate one uninitialized local heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_uninit_heap_shape(
        &mut self,
        trace_map: TraceId,
        heap_site: HeapAllocationSite,
    ) -> Result<HeapReference, Error> {
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(trace_map)?;

        self.heap_mut()
            .allocate_uninit(heap_site, trace_map)
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
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(element.trace_map)?;
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.heap.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.heap.alignment, None, &trace_map);
        let heap = self.heap_mut();

        heap.allocate_dynamic_zeroed(shape).map_err(Error::from)
    }

    /// Allocate one uninitialized local slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_slice(
        &mut self,
        element: AllocationSiteId,
        length: usize,
    ) -> Result<HeapReference, Error> {
        let side_table = self.side_table();
        let element = side_table.allocation_site(element);
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(element.trace_map)?;
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.heap.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.heap.alignment, None, &trace_map);
        let heap = self.heap_mut();

        heap.allocate_dynamic_uninit(shape).map_err(Error::from)
    }

    /// Allocate one byte-initialized local heap payload from one program layout id.
    #[inline(always)]
    pub(crate) fn allocate_heap_layout_bytes(
        &mut self,
        layout_id: mir::LayoutId,
        bytes: &[u8],
    ) -> Result<HeapReference, Error> {
        let program = self.machine.program.clone();
        let shape = program.allocation_shape(layout_id)?;
        let heap = self.heap_mut();

        heap.allocate_dynamic_bytes(shape, bytes)
            .map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one allocation site.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<SharedHeapReference, Error> {
        let (trace_map, heap_site) = self.heap_allocation_site(id);
        self.allocate_zeroed_shared_heap_shape(trace_map, heap_site)
    }

    /// Allocate one zeroed shared small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_small(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<SharedHeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.reserve_shared_small(small_site.small) {
            return Ok(reference);
        }

        self.allocate_zeroed_shared_heap_shape(trace_map, heap_site)
    }

    /// Allocate one zeroed shared heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_zeroed_shared_heap_shape(
        &mut self,
        trace_map: TraceId,
        heap_site: HeapAllocationSite,
    ) -> Result<SharedHeapReference, Error> {
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(trace_map)?;
        let trace_table = program.trace_table();

        self.shared
            .allocate_zeroed(
                self.shared_gc,
                self.shared_cache,
                heap_site,
                trace_map,
                trace_table,
            )
            .map_err(Error::from)
    }

    /// Allocate one uninitialized shared heap payload from one allocation site.
    #[inline(always)]
    pub(crate) fn allocate_uninit_shared_heap(
        &mut self,
        id: AllocationSiteId,
    ) -> Result<SharedHeapReference, Error> {
        let (trace_map, heap_site) = self.heap_allocation_site(id);
        self.allocate_uninit_shared_heap_shape(trace_map, heap_site)
    }

    /// Allocate one uninitialized shared small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_uninit_shared_heap_small(
        &mut self,
        id: SmallAllocationSiteId,
    ) -> Result<SharedHeapReference, Error> {
        let (trace_map, heap_site, small_site) = self.heap_small_allocation_site(id);
        if let Some(reference) = self.reserve_shared_small(small_site.small) {
            return Ok(reference);
        }

        self.allocate_uninit_shared_heap_shape(trace_map, heap_site)
    }

    /// Allocate one uninitialized shared heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_uninit_shared_heap_shape(
        &mut self,
        trace_map: TraceId,
        heap_site: HeapAllocationSite,
    ) -> Result<SharedHeapReference, Error> {
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(trace_map)?;
        let trace_table = program.trace_table();

        self.shared
            .allocate_uninit(
                self.shared_gc,
                self.shared_cache,
                heap_site,
                trace_map,
                trace_table,
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
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(element.trace_map)?;
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.heap.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.heap.alignment, None, &trace_map);

        let trace_table = program.trace_table();

        self.shared
            .allocate_dynamic_zeroed(self.shared_gc, self.shared_cache, shape, trace_table)
            .map_err(Error::from)
    }

    /// Allocate one uninitialized shared slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_uninit_shared_heap_slice(
        &mut self,
        element: AllocationSiteId,
        length: usize,
    ) -> Result<SharedHeapReference, Error> {
        let side_table = self.side_table();
        let element = side_table.allocation_site(element);
        let program = self.machine.program.clone();
        let trace_map = program.trace_map(element.trace_map)?;
        let element_shape = element.shape(trace_map);
        let (byte_len, trace_map) = repeated_layout(
            element_shape.byte_len,
            element.heap.alignment,
            element_shape.trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.heap.alignment, None, &trace_map);

        let trace_table = program.trace_table();

        self.shared
            .allocate_dynamic_uninit(self.shared_gc, self.shared_cache, shape, trace_table)
            .map_err(Error::from)
    }

    /// Return one local heap native address.
    #[inline(always)]
    pub(crate) fn heap_address(&self, reference: HeapReference, byte_offset: usize) -> usize {
        self.heap().heap_base_address() + reference.offset() + byte_offset
    }

    /// Return one shared heap native address.
    #[inline(always)]
    pub(crate) fn shared_heap_address(
        &self,
        reference: SharedHeapReference,
        byte_offset: usize,
    ) -> usize {
        self.shared().heap_base_address() + reference.offset() + byte_offset
    }

    /// Return whether one local heap reference is live.
    #[inline(always)]
    pub(crate) fn is_heap_live(&self, reference: HeapReference) -> bool {
        self.heap().is_heap_live(reference)
    }

    /// Return whether one shared heap reference is live.
    #[inline(always)]
    pub(crate) fn is_shared_heap_live(&self, reference: SharedHeapReference) -> bool {
        self.shared_cache().contains_heap_reference(reference)
            || self.shared().is_heap_live(reference)
    }

    /// Free one local managed allocation.
    #[inline(always)]
    pub(crate) fn free_heap(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap_mut().free(reference)
    }

    /// Free one shared managed allocation.
    #[inline(always)]
    pub(crate) fn free_shared_heap(&mut self, reference: SharedHeapReference) -> HeapResult<()> {
        self.shared.free(self.shared_cache, reference)
    }

    /// Pin one local managed allocation.
    #[inline(always)]
    pub(crate) fn pin_heap(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap_mut().pin(reference)
    }

    /// Unpin one local managed allocation.
    #[inline(always)]
    pub(crate) fn unpin_heap(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap_mut().unpin(reference)
    }

    /// Record one local heap write barrier.
    #[inline(always)]
    pub(crate) fn write_heap_barrier(
        &mut self,
        reference: HeapReference,
        offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let program = self.machine.program.clone();

        self.heap_mut()
            .write_barrier(reference, offset, byte_len, program.trace_table())
    }

    /// Record one shared heap write barrier.
    #[inline(always)]
    pub(crate) fn write_shared_heap_barrier(
        &self,
        reference: SharedHeapReference,
        offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.shared().write_barrier(
            reference,
            offset,
            byte_len,
            self.machine.program.trace_table(),
        )
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
    pub(crate) fn frame_layout(&self) -> &engine::FrameLayout {
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

    /// Return the static address for one global.
    #[inline]
    pub(crate) fn static_address(
        &self,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<StaticAddress> {
        self.statics()
            .address(self.machine.program.static_id(global))
            .or_else(|| self.machine.program.static_address(global))
    }

    /// Resolve one static byte range to a native address.
    #[inline]
    pub(crate) fn static_native_address(
        &self,
        address: StaticAddress,
        byte_len: usize,
    ) -> Result<usize, Error> {
        self.statics()
            .native_address(address, byte_len)
            .or_else(|| {
                self.machine
                    .program
                    .statics
                    .native_address(address, byte_len)
            })
            .ok_or(Error::invalid_instruction())
    }

    /// Resolve one mutable static byte range to a native address.
    #[inline]
    pub(crate) fn static_native_address_mut(
        &mut self,
        address: StaticAddress,
        byte_len: usize,
    ) -> Result<usize, Error> {
        if let Some(native_address) = self.statics.native_address_mut(address, byte_len) {
            return Ok(native_address);
        }

        if self.statics().owns_address_range(address, byte_len) {
            return Err(Error::immutable_global_write(mir::LocalNodeId::new(
                address.id().0,
            )));
        }

        if self
            .machine
            .program
            .statics
            .owns_address_range(address, byte_len)
        {
            return Err(Error::immutable_global_write(mir::LocalNodeId::new(
                address.id().0,
            )));
        }

        Err(Error::invalid_instruction())
    }

    /// Load one SSA value as a VM cell.
    #[inline(always)]
    pub(crate) fn load_value(&self, v: mir::Value) -> Cell {
        let slot = self.value_slot_unchecked(v);

        // cell values live inline in the frame
        if slot.is_cell {
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

    /// Store one cell into an SSA value.
    #[inline(always)]
    pub(crate) fn store_value_cell(&mut self, v: mir::Value, val: Cell) {
        let slot = self.value_slot_unchecked(v);
        let is_cell = slot.is_cell;
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
            .functions
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
