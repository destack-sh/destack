use destack_heap::{
    AllocationCache, AllocationPlan, AllocationShape, Heap, HeapReference, HeapResult, SharedHeap,
    SharedHeapReference, SmallAllocationPlan, TraceView, repeated_layout,
};
use destack_mir::TraceMap;
use destack_program::vm::{AllocationPlanId, SmallAllocationPlanId, TensorLayout};

use super::Activation;
use crate::Cell;
use crate::diagnostic::Error;

impl Activation<'_> {
    /// Borrow the worker heap for this activation.
    #[inline(always)]
    fn heap(&self) -> &Heap {
        self.heap
    }

    /// Borrow the worker heap mutably for this activation.
    #[inline(always)]
    fn heap_mut(&mut self) -> &mut Heap {
        self.heap
    }

    /// Borrow the shared heap for this activation.
    #[inline(always)]
    fn shared(&self) -> &SharedHeap {
        self.shared
    }

    /// Borrow the shared allocation cache for this activation.
    #[inline(always)]
    fn shared_cache(&self) -> &AllocationCache {
        self.shared_cache
    }

    /// Reserve one small shared payload from this activation.
    #[inline(always)]
    fn reserve_shared_small(&mut self, plan: SmallAllocationPlan) -> Option<SharedHeapReference> {
        self.shared
            .reserve_small_from_cache(self.shared_cache, plan.small)
    }

    /// Return one heap allocation plan.
    #[inline(always)]
    fn heap_allocation_plan(&self, id: AllocationPlanId) -> AllocationPlan {
        self.side_table().allocation_plan(self.sections(), id)
    }

    /// Return one small heap allocation plan.
    #[inline(always)]
    fn heap_small_allocation_plan(&self, id: SmallAllocationPlanId) -> SmallAllocationPlan {
        self.side_table().small_allocation_plan(self.sections(), id)
    }

    /// Resolve the trace map for one heap allocation plan.
    fn trace_map_for_allocation<'a>(
        &self,
        allocation: AllocationPlan,
        trace_view: TraceView<'a>,
    ) -> Result<TraceMap, Error> {
        allocation
            .trace_map(trace_view)
            .map_err(|error| Error::internal(error.to_string()))
    }

    /// Allocate one zeroed local heap payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap(
        &mut self,
        id: AllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_allocation_plan(id);

        self.allocate_zeroed_heap_shape(allocation)
    }

    /// Allocate one zeroed local noscan small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_small_noscan(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.heap_mut().reserve_small_noscan(allocation) {
            return Ok(reference);
        }

        self.allocate_zeroed_heap_shape(allocation.allocation)
    }

    /// Allocate one zeroed local scanned small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_small_scan(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.heap_mut().reserve_small_scan(allocation) {
            return Ok(reference);
        }

        self.allocate_zeroed_heap_shape(allocation.allocation)
    }

    /// Allocate one zeroed local small heap payload that may point into shared heap.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_small_shared_edge(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.heap_mut().reserve_small_shared_edge(allocation) {
            return Ok(reference);
        }

        self.allocate_zeroed_heap_shape(allocation.allocation)
    }

    /// Allocate one zeroed local heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_zeroed_heap_shape(
        &mut self,
        allocation: AllocationPlan,
    ) -> Result<HeapReference, Error> {
        let program = self.program;
        let trace_map = self.trace_map_for_allocation(allocation, program.trace_view())?;

        self.heap_mut()
            .allocate_zeroed(allocation, &trace_map)
            .map_err(Error::from)
    }

    /// Allocate one uninitialized local heap payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap(
        &mut self,
        id: AllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_allocation_plan(id);

        self.allocate_uninit_heap_shape(allocation)
    }

    /// Allocate one uninitialized local noscan small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_small_noscan(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.heap_mut().reserve_small_noscan(allocation) {
            return Ok(reference);
        }

        self.allocate_uninit_heap_shape(allocation.allocation)
    }

    /// Allocate one uninitialized local scanned small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_small_scan(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.heap_mut().reserve_small_scan(allocation) {
            return Ok(reference);
        }

        self.allocate_uninit_heap_shape(allocation.allocation)
    }

    /// Allocate one uninitialized local small heap payload that may point into shared heap.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_small_shared_edge(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<HeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.heap_mut().reserve_small_shared_edge(allocation) {
            return Ok(reference);
        }

        self.allocate_uninit_heap_shape(allocation.allocation)
    }

    /// Allocate one uninitialized local heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_uninit_heap_shape(
        &mut self,
        allocation: AllocationPlan,
    ) -> Result<HeapReference, Error> {
        let program = self.program;
        let trace_map = self.trace_map_for_allocation(allocation, program.trace_view())?;

        self.heap_mut()
            .allocate_uninit(allocation, &trace_map)
            .map_err(Error::from)
    }

    /// Allocate one zeroed local slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_slice(
        &mut self,
        element: AllocationPlanId,
        length: usize,
    ) -> Result<HeapReference, Error> {
        let program = self.program;
        let element = self.heap_allocation_plan(element);
        let element_trace_map = self.trace_map_for_allocation(element, program.trace_view())?;
        let (byte_len, trace_map) = repeated_layout(
            element.byte_len(),
            element.alignment(),
            &element_trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment(), None, trace_map);
        let plan = self.heap().options().allocation_plan(&shape);
        let heap = self.heap_mut();

        heap.allocate_zeroed(plan, &shape.trace_map)
            .map_err(Error::from)
    }

    /// Allocate one uninitialized local slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_uninit_heap_slice(
        &mut self,
        element: AllocationPlanId,
        length: usize,
    ) -> Result<HeapReference, Error> {
        let program = self.program;
        let element = self.heap_allocation_plan(element);
        let element_trace_map = self.trace_map_for_allocation(element, program.trace_view())?;
        let (byte_len, trace_map) = repeated_layout(
            element.byte_len(),
            element.alignment(),
            &element_trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment(), None, trace_map);
        let plan = self.heap().options().allocation_plan(&shape);
        let heap = self.heap_mut();

        heap.allocate_uninit(plan, &shape.trace_map)
            .map_err(Error::from)
    }

    /// Allocate one zeroed local tensor storage.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_heap_tensor(
        &mut self,
        layout: &TensorLayout,
    ) -> Result<HeapReference, Error> {
        let trace_map = TraceMap::empty();
        let shape = AllocationShape::new(layout.byte_len(), Cell::BYTE_LEN, None, trace_map);
        let plan = self.heap().options().allocation_plan(&shape);
        let heap = self.heap_mut();

        heap.allocate_zeroed(plan, &shape.trace_map)
            .map_err(Error::from)
    }

    /// Allocate one zeroed shared heap payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap(
        &mut self,
        id: AllocationPlanId,
    ) -> Result<SharedHeapReference, Error> {
        let allocation = self.heap_allocation_plan(id);

        self.allocate_zeroed_shared_heap_shape(allocation)
    }

    /// Allocate one zeroed shared small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_small(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<SharedHeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.reserve_shared_small(allocation) {
            return Ok(reference);
        }

        self.allocate_zeroed_shared_heap_shape(allocation.allocation)
    }

    /// Allocate one zeroed shared heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_zeroed_shared_heap_shape(
        &mut self,
        allocation: AllocationPlan,
    ) -> Result<SharedHeapReference, Error> {
        let program = self.program;
        let trace_map = self.trace_map_for_allocation(allocation, program.trace_view())?;
        let trace_view = program.trace_view();

        self.shared
            .allocate_zeroed(
                self.shared_mark_worker,
                self.shared_cache,
                allocation,
                &trace_map,
                trace_view,
            )
            .map_err(Error::from)
    }

    /// Allocate one uninitialized shared heap payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_uninit_shared_heap(
        &mut self,
        id: AllocationPlanId,
    ) -> Result<SharedHeapReference, Error> {
        let allocation = self.heap_allocation_plan(id);

        self.allocate_uninit_shared_heap_shape(allocation)
    }

    /// Allocate one uninitialized shared small heap payload.
    #[inline(always)]
    pub(crate) fn allocate_uninit_shared_heap_small(
        &mut self,
        id: SmallAllocationPlanId,
    ) -> Result<SharedHeapReference, Error> {
        let allocation = self.heap_small_allocation_plan(id);
        if let Some(reference) = self.reserve_shared_small(allocation) {
            return Ok(reference);
        }

        self.allocate_uninit_shared_heap_shape(allocation.allocation)
    }

    /// Allocate one uninitialized shared heap payload from one decoded shape.
    #[cold]
    #[inline(never)]
    fn allocate_uninit_shared_heap_shape(
        &mut self,
        allocation: AllocationPlan,
    ) -> Result<SharedHeapReference, Error> {
        let program = self.program;
        let trace_map = self.trace_map_for_allocation(allocation, program.trace_view())?;
        let trace_view = program.trace_view();

        self.shared
            .allocate_uninit(
                self.shared_mark_worker,
                self.shared_cache,
                allocation,
                &trace_map,
                trace_view,
            )
            .map_err(Error::from)
    }

    /// Allocate one zeroed shared slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_zeroed_shared_heap_slice(
        &mut self,
        element: AllocationPlanId,
        length: usize,
    ) -> Result<SharedHeapReference, Error> {
        let program = self.program;
        let element = self.heap_allocation_plan(element);
        let element_trace_map = self.trace_map_for_allocation(element, program.trace_view())?;
        let (byte_len, trace_map) = repeated_layout(
            element.byte_len(),
            element.alignment(),
            &element_trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment(), None, trace_map);
        let plan = self.shared.options().allocation_plan(&shape);
        let trace_view = program.trace_view();

        self.shared
            .allocate_zeroed(
                self.shared_mark_worker,
                self.shared_cache,
                plan,
                &shape.trace_map,
                trace_view,
            )
            .map_err(Error::from)
    }

    /// Allocate one uninitialized shared slice backing array.
    #[inline(always)]
    pub(crate) fn allocate_uninit_shared_heap_slice(
        &mut self,
        element: AllocationPlanId,
        length: usize,
    ) -> Result<SharedHeapReference, Error> {
        let program = self.program;
        let element = self.heap_allocation_plan(element);
        let element_trace_map = self.trace_map_for_allocation(element, program.trace_view())?;
        let (byte_len, trace_map) = repeated_layout(
            element.byte_len(),
            element.alignment(),
            &element_trace_map,
            length,
        )?;
        let shape = AllocationShape::new(byte_len, element.alignment(), None, trace_map);
        let plan = self.shared.options().allocation_plan(&shape);
        let trace_view = program.trace_view();

        self.shared
            .allocate_uninit(
                self.shared_mark_worker,
                self.shared_cache,
                plan,
                &shape.trace_map,
                trace_view,
            )
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
        let trace_view = self.program.trace_view();

        self.heap_mut()
            .write_barrier(reference, offset, byte_len, trace_view)
    }

    /// Record one shared heap write barrier.
    #[inline(always)]
    pub(crate) fn write_shared_heap_barrier(
        &self,
        reference: SharedHeapReference,
        offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.shared()
            .write_barrier(reference, offset, byte_len, self.program.trace_view())
    }
}
