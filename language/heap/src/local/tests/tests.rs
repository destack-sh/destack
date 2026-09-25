use std::sync::{Arc, OnceLock};

use tspp_core::{SectionBuilder, SectionImage, SectionStorage};
use tspp_memory::MemoryMap;

use crate::{TraceTable, TraceView};

use crate::local::storage::HeapStorage;
use crate::local::{Heap, HeapLimits, HeapOptions};
use crate::{Allocation, AllocationClass, AllocationPlan, AllocationShape, HeapReference, Payload};

static TRACE_TABLE: OnceLock<TestTraceTable> = OnceLock::new();

/// Section-backed trace table used by heap tests.
pub(crate) struct TestTraceTable {
    /// Packed section storage.
    storage: SectionStorage,
    /// Packed heap trace table.
    traces: TraceTable,
}

/// A local heap layer that can build plans and allocate blocks for tests.
pub(crate) trait TestHeapPlan {
    /// Build one allocation plan for this test heap layer.
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a>;

    /// Allocate one block for this test heap layer.
    fn test_allocate(&mut self, shape: AllocationShape, payload: Payload<'_>) -> HeapReference;
}

impl TestHeapPlan for Heap {
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a> {
        let plan = self.options().allocation_plan(shape);

        plan.allocation(&shape.trace_map)
    }

    fn test_allocate(&mut self, shape: AllocationShape, payload: Payload<'_>) -> HeapReference {
        let plan = self.test_allocation_plan(&shape);

        self.allocate_payload(&plan, payload)
            .expect("test block should allocate")
    }
}

impl<T> TestHeapPlan for &mut T
where
    T: TestHeapPlan + ?Sized,
{
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a> {
        (**self).test_allocation_plan(shape)
    }

    fn test_allocate(&mut self, shape: AllocationShape, payload: Payload<'_>) -> HeapReference {
        (**self).test_allocate(shape, payload)
    }
}

impl TestHeapPlan for HeapStorage {
    fn test_allocation_plan<'a>(&self, shape: &'a AllocationShape) -> Allocation<'a> {
        let class = if shape.requires_individual_metadata() {
            AllocationClass::large()
        } else {
            AllocationClass::select(
                shape.byte_len,
                shape.alignment,
                shape.trace_id,
                shape.drop,
                shape.is_noscan,
                &self.small.size_classes,
                self.page_size_bytes(),
                self.small.span_size_bytes,
            )
        };
        let plan = AllocationPlan::new(shape, class);

        plan.allocation(&shape.trace_map)
    }

    fn test_allocate(&mut self, shape: AllocationShape, payload: Payload<'_>) -> HeapReference {
        let plan = self.test_allocation_plan(&shape);

        self.allocate(&plan, payload)
            .expect("test block should allocate")
    }
}

/// Build one local heap from explicit limits and options.
pub(crate) fn test_heap_with_limits(limits: HeapLimits, options: HeapOptions) -> Heap {
    let memory = test_memory(options.page_size_bytes);

    Heap::new(memory, limits, options).expect("test heap should build")
}

/// Build one local heap from explicit options.
pub(crate) fn test_heap(options: HeapOptions) -> Heap {
    test_heap_with_limits(HeapLimits::default(), options)
}

/// Build one local heap storage from explicit options.
pub(crate) fn test_storage(options: &HeapOptions) -> HeapStorage {
    let memory = test_memory(options.page_size_bytes);

    HeapStorage::new(memory, options).expect("test heap storage should build")
}

/// Reserve one World memory map for heap tests.
pub(crate) fn test_memory(page_size_bytes: usize) -> Arc<MemoryMap> {
    Arc::new(
        MemoryMap::reserve(1024 * 1024 * 1024, page_size_bytes)
            .expect("test World memory should reserve"),
    )
}

/// Return the shared empty trace table for heap tests.
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

/// Read bytes from one mapped heap address.
pub(crate) fn read_mapped_bytes(address: usize, byte_len: usize) -> Vec<u8> {
    // SAFETY: tests only read ranges they just allocated or restored
    unsafe { std::slice::from_raw_parts(address as *const u8, byte_len).to_vec() }
}

/// Write one byte to one mapped heap address.
pub(crate) fn write_mapped_byte(address: usize, byte: u8) {
    // SAFETY: tests only write ranges they just allocated or restored
    unsafe {
        std::ptr::write(address as *mut u8, byte);
    }
}

/// Write bytes to one mapped heap address.
pub(crate) fn write_mapped_bytes(address: usize, bytes: &[u8]) {
    // SAFETY: tests only write ranges they just allocated or restored
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
}
