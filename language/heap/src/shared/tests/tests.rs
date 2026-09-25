use std::sync::{Arc, OnceLock};

use tspp_core::{SectionBuilder, SectionImage, SectionStorage};
use tspp_memory::MemoryMap;

use crate::{TraceTable, TraceView};

use crate::shared::storage::HeapStorage;
use crate::{
    Allocation, AllocationCache, AllocationClass, AllocationPlan, AllocationShape, Payload,
    SharedHeap, SharedHeapReference, SharedMarkWorker,
};

static TRACE_TABLE: OnceLock<TestTraceTable> = OnceLock::new();

/// Section-backed trace table used by shared heap tests.
pub(crate) struct TestTraceTable {
    /// Packed section storage.
    storage: SectionStorage,
    /// Packed heap trace table.
    traces: TraceTable,
}

/// Reserve one World memory map for shared heap tests.
pub(crate) fn test_memory(page_size_bytes: usize) -> Arc<MemoryMap> {
    Arc::new(
        MemoryMap::reserve(1024 * 1024 * 1024, page_size_bytes)
            .expect("test World memory should reserve"),
    )
}

/// A shared heap layer that can build allocation plans for tests.
pub(crate) trait TestHeapPlan {
    /// Build one allocation plan for this test heap layer.
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a>;
}

impl TestHeapPlan for SharedHeap {
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a> {
        let plan = self.options().allocation_plan(shape);

        plan.allocation(&shape.trace_map)
    }
}

impl<T> TestHeapPlan for &mut T
where
    T: TestHeapPlan + ?Sized,
{
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a> {
        (**self).test_allocation_plan(shape)
    }
}

impl TestHeapPlan for HeapStorage {
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a> {
        let store = self.state.read();
        let class = if shape.trace_map.has_variant_reference() {
            AllocationClass::large()
        } else {
            AllocationClass::select(
                shape.byte_len,
                shape.alignment,
                shape.trace_id,
                shape.drop,
                shape.is_noscan,
                &store.small.size_classes,
                self.page_size_bytes(),
                store.small.span_size_bytes,
            )
        };
        let plan = AllocationPlan::new(shape, class);

        plan.allocation(&shape.trace_map)
    }
}

/// Return the shared empty trace table for shared heap tests.
pub(crate) fn trace_view() -> TraceView<'static> {
    TRACE_TABLE.get_or_init(TestTraceTable::new).view()
}

impl TestTraceTable {
    /// Build one empty section-backed trace table.
    pub(crate) fn new() -> Self {
        Self::from_mir(&tspp_mir::TraceTable::new())
    }

    /// Build one section-backed trace table from MIR traces.
    pub(crate) fn from_mir(source: &tspp_mir::TraceTable) -> Self {
        let mut sections = SectionBuilder::new();
        let traces = TraceTable::pack(&mut sections, source);
        let storage = sections.build();

        Self { storage, traces }
    }

    /// Return the packed trace view.
    pub(crate) fn view(&self) -> TraceView<'_> {
        // SAFETY: test storage is built exclusively through SectionBuilder.
        let sections = unsafe { SectionImage::new(&self.storage) };

        self.traces.view(sections)
    }
}

/// Allocate one shared block through one worker-local cache.
pub(crate) fn test_allocate(
    shared: &SharedHeap,
    worker: &SharedMarkWorker,
    cache: &mut AllocationCache,
    shape: AllocationShape,
    payload: Payload<'_>,
) -> SharedHeapReference {
    let plan = shared.test_allocation_plan(&shape);

    shared
        .allocate_payload(worker, cache, &plan, payload, trace_view())
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
