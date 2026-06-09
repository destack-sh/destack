use std::sync::OnceLock;

use destack_mir::TraceTable;

use crate::shared::storage::HeapStorage;
use crate::{
    AllocationCache, AllocationClass, AllocationPlan, AllocationShape, AllocationSite, GcWorker,
    Payload, SharedHeap, SharedHeapOptions, SharedHeapReference, allocation_class,
};

static TRACE_TABLE: OnceLock<TraceTable> = OnceLock::new();

/// A shared heap layer that can build allocation plans for tests.
pub(crate) trait TestHeapPlan {
    /// Build one allocation plan for this test heap layer.
    fn test_allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a>;
}

impl TestHeapPlan for SharedHeap {
    fn test_allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        allocation_plan(self.options(), shape)
    }
}

impl<T> TestHeapPlan for &mut T
where
    T: TestHeapPlan + ?Sized,
{
    fn test_allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        (**self).test_allocation_plan(shape)
    }
}

impl TestHeapPlan for HeapStorage {
    fn test_allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        let store = self.state.read();
        let class = if shape.trace_map.has_tagged_reference() {
            AllocationClass::Large
        } else {
            allocation_class(
                shape.byte_len,
                shape.alignment,
                shape.trace_id,
                shape.is_noscan,
                &store.small.size_classes,
                self.allocator.page_size_bytes(),
                store.small.span_size_bytes,
            )
        };
        let site = AllocationSite::new(shape, class);

        site.plan(shape.trace_map)
    }
}

/// Return the shared empty trace table for shared heap tests.
pub(crate) fn trace_table() -> &'static TraceTable {
    TRACE_TABLE.get_or_init(TraceTable::new)
}

/// Build one explicit shared heap allocation site.
pub(crate) fn allocation_site(
    options: &SharedHeapOptions,
    shape: AllocationShape<'_>,
) -> AllocationSite {
    options.allocation_site_for_shape(shape)
}

/// Build one shared heap allocation plan.
pub(crate) fn allocation_plan<'a>(
    options: &SharedHeapOptions,
    shape: AllocationShape<'a>,
) -> AllocationPlan<'a> {
    let site = allocation_site(options, shape);

    site.plan(shape.trace_map)
}

/// Build one allocation plan for a live shared test heap.
pub(crate) fn heap_allocation_plan<'a>(
    heap: &impl TestHeapPlan,
    shape: AllocationShape<'a>,
) -> AllocationPlan<'a> {
    heap.test_allocation_plan(shape)
}

/// Allocate one shared block through one worker-local cache.
pub(crate) fn test_allocate(
    shared: &SharedHeap,
    worker: &GcWorker,
    cache: &mut AllocationCache,
    shape: AllocationShape<'_>,
    payload: Payload<'_>,
) -> SharedHeapReference {
    let plan = heap_allocation_plan(shared, shape);

    shared
        .allocate_payload(worker, cache, &plan, payload, trace_table())
        .expect("shared test block should allocate")
}

/// Read bytes from one mapped shared heap address.
pub(crate) fn read_mapped_bytes(address: usize, byte_len: usize) -> Vec<u8> {
    // SAFETY: tests only read ranges they just allocated or restored
    unsafe { std::slice::from_raw_parts(address as *const u8, byte_len).to_vec() }
}

/// Write bytes to one mapped shared heap address.
pub(crate) fn write_mapped_bytes(address: usize, bytes: &[u8]) {
    // SAFETY: tests only write ranges they just allocated or restored
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
}
