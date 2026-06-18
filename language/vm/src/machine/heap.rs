use destack_heap::{
    AllocationCache, AllocationShape, AllocationSite, Heap, HeapReference, HeapResult, SharedHeap,
    SharedHeapReference, SmallAllocationPlan, SmallAllocationSite, repeated_layout,
};
use destack_mir::{LayoutId, TraceId};
use destack_program::vm::{AllocationSiteId, SmallAllocationSiteId};

use super::Activation;
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
            .reserve_small_from_cache(self.shared_cache, plan)
    }

    /// Return one trace map id and heap allocation site.
    #[inline(always)]
    fn heap_allocation_site(&self, id: AllocationSiteId) -> (TraceId, AllocationSite) {
        let side_table = self.side_table();
        let vm_site = *side_table.allocation_site(id);

        (vm_site.trace_map, vm_site.heap)
    }

    /// Return one trace map id, cold heap site, and small heap site.
    #[inline(always)]
    fn heap_small_allocation_site(
        &self,
        id: SmallAllocationSiteId,
    ) -> (TraceId, AllocationSite, SmallAllocationSite) {
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
        heap_site: AllocationSite,
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
        heap_site: AllocationSite,
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
        let site = self.heap().options().allocation_site_for_shape(shape);
        let heap = self.heap_mut();

        heap.allocate_zeroed(site, shape.trace_map)
            .map_err(Error::from)
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
        let site = self.heap().options().allocation_site_for_shape(shape);
        let heap = self.heap_mut();

        heap.allocate_uninit(site, shape.trace_map)
            .map_err(Error::from)
    }

    /// Allocate one byte-initialized local heap payload from one program layout id.
    #[inline(always)]
    pub(crate) fn allocate_heap_layout_bytes(
        &mut self,
        layout_id: LayoutId,
        bytes: &[u8],
    ) -> Result<HeapReference, Error> {
        let program = self.machine.program.clone();
        let shape = program.allocation_shape(layout_id)?;
        let site = self.heap().options().allocation_site_for_shape(shape);
        let heap = self.heap_mut();

        heap.allocate_bytes(site, shape.trace_map, bytes)
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
        heap_site: AllocationSite,
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
        heap_site: AllocationSite,
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
        let site = self.shared.options().allocation_site_for_shape(shape);
        let trace_table = program.trace_table();

        self.shared
            .allocate_zeroed(
                self.shared_gc,
                self.shared_cache,
                site,
                shape.trace_map,
                trace_table,
            )
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
        let site = self.shared.options().allocation_site_for_shape(shape);
        let trace_table = program.trace_table();

        self.shared
            .allocate_uninit(
                self.shared_gc,
                self.shared_cache,
                site,
                shape.trace_map,
                trace_table,
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
}
