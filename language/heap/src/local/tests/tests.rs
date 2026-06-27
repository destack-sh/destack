use std::sync::OnceLock;

use destack_mir::TraceTable;

use crate::local::storage::HeapStorage;
use crate::local::{Heap, HeapLimits, HeapOptions};
use crate::{
    Allocation, AllocationClass, AllocationPlan, HeapReference, Payload, PayloadShape,
    allocation_class, test_allocator,
};

static TRACE_TABLE: OnceLock<TraceTable> = OnceLock::new();

/// A local heap layer that can build plans and allocate blocks for tests.
pub(crate) trait TestHeapPlan {
    /// Build one allocation plan for this test heap layer.
    fn test_allocation_plan<'a>(&self, shape: PayloadShape<'a>) -> Allocation<'a>;

    /// Allocate one block for this test heap layer.
    fn test_allocate(&mut self, shape: PayloadShape<'_>, payload: Payload<'_>) -> HeapReference;
}

impl TestHeapPlan for Heap {
    fn test_allocation_plan<'a>(&self, shape: PayloadShape<'a>) -> Allocation<'a> {
        allocation_plan(self.options(), shape)
    }

    fn test_allocate(&mut self, shape: PayloadShape<'_>, payload: Payload<'_>) -> HeapReference {
        let plan = self.test_allocation_plan(shape);

        self.allocate_payload(&plan, payload)
            .expect("test block should allocate")
    }
}

impl<T> TestHeapPlan for &mut T
where
    T: TestHeapPlan + ?Sized,
{
    fn test_allocation_plan<'a>(&self, shape: PayloadShape<'a>) -> Allocation<'a> {
        (**self).test_allocation_plan(shape)
    }

    fn test_allocate(&mut self, shape: PayloadShape<'_>, payload: Payload<'_>) -> HeapReference {
        (**self).test_allocate(shape, payload)
    }
}

impl TestHeapPlan for HeapStorage {
    fn test_allocation_plan<'a>(&self, shape: PayloadShape<'a>) -> Allocation<'a> {
        let class = if shape.trace_map.has_tagged_reference() {
            AllocationClass::Large
        } else {
            allocation_class(
                shape.byte_len,
                shape.alignment,
                shape.trace_id,
                shape.is_noscan,
                &self.small.size_classes,
                self.allocator().page_size_bytes(),
                self.small.span_size_bytes,
            )
        };
        let plan = AllocationPlan::new(shape, class);

        plan.allocation(shape.trace_map)
    }

    fn test_allocate(&mut self, shape: PayloadShape<'_>, payload: Payload<'_>) -> HeapReference {
        let plan = self.test_allocation_plan(shape);

        self.allocate(&plan, payload)
            .expect("test block should allocate")
    }
}

/// Build one local heap from explicit limits and options.
pub(crate) fn test_heap_with_limits(limits: HeapLimits, options: HeapOptions) -> Heap {
    let allocator = test_allocator(&options);

    Heap::with_allocator_limits_and_options(allocator, limits, options)
        .expect("test heap should build")
}

/// Build one local heap from explicit options.
pub(crate) fn test_heap(options: HeapOptions) -> Heap {
    test_heap_with_limits(HeapLimits::default(), options)
}

/// Build one local heap storage from explicit options.
pub(crate) fn test_storage(options: &HeapOptions) -> HeapStorage {
    let allocator = test_allocator(options);

    HeapStorage::build_with_options(allocator, options).expect("test heap storage should build")
}

/// Return the shared empty trace table for heap tests.
pub(crate) fn trace_table() -> &'static TraceTable {
    TRACE_TABLE.get_or_init(TraceTable::new)
}

/// Build one explicit local heap allocation plan.
pub(crate) fn owned_allocation_plan(
    options: &HeapOptions,
    shape: PayloadShape<'_>,
) -> AllocationPlan {
    options.allocation_plan_for_shape(shape)
}

/// Build one local heap allocation plan.
pub(crate) fn allocation_plan<'a>(
    options: &HeapOptions,
    shape: PayloadShape<'a>,
) -> Allocation<'a> {
    let plan = owned_allocation_plan(options, shape);

    plan.allocation(shape.trace_map)
}

/// Build one allocation plan for a live test heap.
pub(crate) fn heap_allocation_plan<'a>(
    heap: &impl TestHeapPlan,
    shape: PayloadShape<'a>,
) -> Allocation<'a> {
    heap.test_allocation_plan(shape)
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
