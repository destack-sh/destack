use std::sync::OnceLock;

use destack_core::{SectionDirectory, SectionImage, SectionPacker, SectionStorage};

use crate::{TraceTable, TraceView};

use crate::shared::storage::HeapStorage;
use crate::{
    Allocation, AllocationCache, AllocationClass, AllocationPlan, AllocationShape, Payload,
    SharedHeap, SharedHeapOptions, SharedHeapReference, SharedMarkWorker, allocation_class,
};

static TRACE_TABLE: OnceLock<TestTraceTable> = OnceLock::new();

/// Section-backed trace table used by shared heap tests.
pub(crate) struct TestTraceTable {
    /// Packed section directory.
    sections: SectionDirectory,
    /// Packed section storage.
    storage: SectionStorage,
    /// Packed heap trace table.
    traces: TraceTable,
}

/// A shared heap layer that can build allocation plans for tests.
pub(crate) trait TestHeapPlan {
    /// Build one allocation plan for this test heap layer.
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a>;
}

impl TestHeapPlan for SharedHeap {
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a> {
        allocation_plan(self.options(), shape)
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
        let class = if shape.trace_map.has_tagged_reference() {
            AllocationClass::large()
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
        let plan = AllocationPlan::new(&shape, class);

        plan.allocation(&shape.trace_map)
    }
}

/// Return the shared empty trace table for shared heap tests.
pub(crate) fn trace_view() -> TraceView<'static> {
    TRACE_TABLE.get_or_init(TestTraceTable::new).view()
}

/// Build one explicit shared heap allocation plan.
pub(crate) fn owned_allocation_plan(
    options: &SharedHeapOptions,
    shape: &AllocationShape,
) -> AllocationPlan {
    options.allocation_plan(shape)
}

/// Build one shared heap allocation plan.
pub(crate) fn allocation_plan<'a>(
    options: &SharedHeapOptions,
    shape: &'a AllocationShape,
) -> Allocation<'a> {
    let plan = owned_allocation_plan(options, shape);

    plan.allocation(&shape.trace_map)
}

/// Build one allocation plan for a live shared test heap.
pub(crate) fn heap_allocation_plan<'a>(
    heap: &impl TestHeapPlan,
    shape: &'a AllocationShape,
) -> Allocation<'a> {
    heap.test_allocation_plan(shape)
}

impl TestTraceTable {
    /// Build one empty section-backed trace table.
    pub(crate) fn new() -> Self {
        Self::from_mir(&destack_mir::TraceTable::new())
    }

    /// Build one section-backed trace table from MIR traces.
    pub(crate) fn from_mir(source: &destack_mir::TraceTable) -> Self {
        let mut sections = SectionPacker::new();
        let traces = TraceTable::pack(&mut sections, source);
        let (sections, storage) = sections.finish();

        Self {
            sections,
            storage,
            traces,
        }
    }

    /// Return the packed trace view.
    pub(crate) fn view(&self) -> TraceView<'_> {
        let sections = SectionImage::load(&self.sections, &self.storage)
            .expect("test trace sections should load");

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
    let plan = heap_allocation_plan(shared, &shape);

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
