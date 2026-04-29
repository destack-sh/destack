use std::sync::Arc;

use destack_heap::{
    AddressSpace, AllocationLayout, Allocator, DEFAULT_PAGE_BYTES, Heap, HeapLimits, HeapOptions,
    HeapReference, SharedAllocator, SharedHeap, SharedHeapLimits, SharedHeapReference,
};
use destack_mir::ReferenceMap;

/// The virtual memory range reserved by address-space benchmarks.
pub(crate) const SPACE_BYTES: usize = 16 * 1024 * 1024;
/// The operating-system page width used by the heap allocator.
pub(crate) const PAGE_BYTES: usize = DEFAULT_PAGE_BYTES;
/// The number of small allocations in one wrapper-path sample.
pub(crate) const SMALL_ALLOCATIONS: usize = 1024;
/// The representative small object payload width.
pub(crate) const SMALL_BYTES: usize = 32;
/// The number of large allocations in one large-object sample.
pub(crate) const LARGE_ALLOCATIONS: usize = 64;
/// The representative large object payload width.
pub(crate) const LARGE_BYTES: usize = 1024 * 1024;
/// The byte width copied by bulk access benchmarks.
pub(crate) const ACCESS_BYTES: usize = 256 * 1024;
/// The number of bulk access passes in one sample.
pub(crate) const ACCESS_PASSES: usize = 128;
/// The native pointer width used by heap references.
pub(crate) const REFERENCE_BYTES: usize = std::mem::size_of::<usize>();
/// The number of pointer-width words in one access payload.
pub(crate) const ACCESS_WORDS: usize = ACCESS_BYTES / REFERENCE_BYTES;
/// The number of scalar access passes in one sample.
pub(crate) const SCALAR_ACCESS_PASSES: usize = 16;
/// The number of record field access passes in one sample.
pub(crate) const FIELD_ACCESS_PASSES: usize = 128;
/// The number of hot-path allocations in one matrix sample.
pub(crate) const MATRIX_ALLOCATIONS: usize = 8 * 1024;
/// The number of allocations performed by each parallel worker.
pub(crate) const PARALLEL_ALLOCATIONS_PER_WORKER: usize = 4 * 1024;
/// The number of records in one object-graph workload.
pub(crate) const WORKLOAD_OBJECTS: usize = 512;
/// The number of records mutated after one fork.
pub(crate) const WORKLOAD_MUTATIONS: usize = 128;
/// The byte width of one record payload.
pub(crate) const RECORD_BYTES: usize = 32;
/// The byte width of one leaf payload.
pub(crate) const LEAF_BYTES: usize = 24;
/// The allocation sizes sampled by the hot-path matrix.
pub(crate) const ALLOCATION_MATRIX_BYTES: &[usize] =
    &[8, 16, 24, 32, 64, 128, 256, 512, 1024, 4096, 32_768];
/// The worker counts sampled by shared allocation benchmarks.
pub(crate) const PARALLEL_WORKERS: &[usize] = &[1, 2, 4, 8];
/// The materialized page counts sampled by fork benchmarks.
pub(crate) const FORK_MATERIALIZED_PAGES: &[usize] = &[1, 16, 256];
/// The byte width touched before raw address-copy benchmarks.
pub(crate) const PRETOUCH_BYTES: usize = MATRIX_ALLOCATIONS * SMALL_BYTES;

/// One shared heap with one worker-local allocator.
pub(crate) struct SharedFixture {
    /// The shared heap under test.
    pub(crate) heap: SharedHeap,
    /// The worker-local allocation front end.
    pub(crate) allocator: SharedAllocator,
}

/// Reserve one address space for memory benchmarks.
pub(crate) fn reserve_space() -> AddressSpace {
    AddressSpace::reserve(SPACE_BYTES, PAGE_BYTES).expect("address space should reserve")
}

/// Reserve one address space and materialize the requested page count.
pub(crate) fn materialized_space(page_count: usize) -> AddressSpace {
    let space = reserve_space();
    let page = vec![0xAB; PAGE_BYTES];

    for page_index in 0..page_count {
        let offset = page_index * PAGE_BYTES;
        space
            .write(offset, &page)
            .expect("address space page write should succeed");
    }

    space
}

/// Build one local heap suitable for allocation benchmarks.
pub(crate) fn local_heap() -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("heap should build")
}

/// Build one shared heap and worker-local allocator.
pub(crate) fn shared_fixture() -> SharedFixture {
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let allocator = shared.allocator();

    SharedFixture {
        heap: shared,
        allocator,
    }
}

/// Encode one machine word into one payload buffer.
pub(crate) fn write_word(bytes: &mut [u8], offset: usize, value: usize) {
    bytes[offset..offset + REFERENCE_BYTES].copy_from_slice(&value.to_le_bytes());
}

/// Build the scan map for one record with a local reference field.
pub(crate) fn local_record_reference_map() -> ReferenceMap {
    ReferenceMap::Reference {
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
    }
}

/// Build the scan map for one record with a shared reference field.
pub(crate) fn shared_record_reference_map() -> ReferenceMap {
    ReferenceMap::Reference {
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
    }
}

/// Build the scan map for one local reference array.
pub(crate) fn local_reference_array_map() -> ReferenceMap {
    ReferenceMap::RepeatedReference {
        count: WORKLOAD_OBJECTS as u32,
        stride: REFERENCE_BYTES as u32,
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
    }
}

/// Build the scan map for one shared reference array.
pub(crate) fn shared_reference_array_map() -> ReferenceMap {
    ReferenceMap::RepeatedReference {
        count: WORKLOAD_OBJECTS as u32,
        stride: REFERENCE_BYTES as u32,
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
    }
}

/// Allocate one local object graph shaped like records pointing at leaf records.
pub(crate) fn allocate_local_object_graph(heap: &mut Heap) -> Vec<HeapReference> {
    let leaf_map = ReferenceMap::None;
    let record_map = local_record_reference_map();
    let leaf_layout = AllocationLayout::new(LEAF_BYTES, &leaf_map);
    let record_layout = AllocationLayout::new(RECORD_BYTES, &record_map);
    let mut records = Vec::with_capacity(WORKLOAD_OBJECTS);

    for index in 0..WORKLOAD_OBJECTS {
        let leaf = heap
            .allocate_zeroed(leaf_layout)
            .expect("leaf allocation should succeed");
        let mut record = [0u8; RECORD_BYTES];
        write_word(&mut record, 0, leaf.bits());
        write_word(&mut record, REFERENCE_BYTES, index);

        let reference = heap
            .allocate_bytes(record_layout, &record)
            .expect("record allocation should succeed");
        records.push(reference);
    }

    records
}

/// Allocate one shared object graph shaped like records pointing at leaf records.
pub(crate) fn allocate_shared_object_graph(
    shared: &SharedHeap,
    allocator: &mut SharedAllocator,
) -> Vec<SharedHeapReference> {
    let leaf_map = ReferenceMap::None;
    let record_map = shared_record_reference_map();
    let leaf_layout = AllocationLayout::new(LEAF_BYTES, &leaf_map);
    let record_layout = AllocationLayout::new(RECORD_BYTES, &record_map);
    let mut records = Vec::with_capacity(WORKLOAD_OBJECTS);

    for index in 0..WORKLOAD_OBJECTS {
        let leaf = shared
            .allocate_zeroed(allocator, leaf_layout)
            .expect("shared leaf allocation should succeed");
        let mut record = [0u8; RECORD_BYTES];
        write_word(&mut record, 0, leaf.bits());
        write_word(&mut record, REFERENCE_BYTES, index);

        let reference = shared
            .allocate_bytes(allocator, record_layout, &record)
            .expect("shared record allocation should succeed");
        records.push(reference);
    }

    records
}

/// Allocate one local reference array backed by a repeated reference map.
pub(crate) fn allocate_local_reference_array(heap: &mut Heap) -> HeapReference {
    let leaf_map = ReferenceMap::None;
    let array_map = local_reference_array_map();
    let leaf_layout = AllocationLayout::new(LEAF_BYTES, &leaf_map);
    let array_layout = AllocationLayout::new(WORKLOAD_OBJECTS * REFERENCE_BYTES, &array_map);
    let mut payload = vec![0u8; WORKLOAD_OBJECTS * REFERENCE_BYTES];

    for index in 0..WORKLOAD_OBJECTS {
        let leaf = heap
            .allocate_zeroed(leaf_layout)
            .expect("leaf allocation should succeed");
        write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
    }

    heap.allocate_bytes(array_layout, &payload)
        .expect("reference array allocation should succeed")
}

/// Allocate one shared reference array backed by a repeated reference map.
pub(crate) fn allocate_shared_reference_array(
    shared: &SharedHeap,
    allocator: &mut SharedAllocator,
) -> SharedHeapReference {
    let leaf_map = ReferenceMap::None;
    let array_map = shared_reference_array_map();
    let leaf_layout = AllocationLayout::new(LEAF_BYTES, &leaf_map);
    let array_layout = AllocationLayout::new(WORKLOAD_OBJECTS * REFERENCE_BYTES, &array_map);
    let mut payload = vec![0u8; WORKLOAD_OBJECTS * REFERENCE_BYTES];

    for index in 0..WORKLOAD_OBJECTS {
        let leaf = shared
            .allocate_zeroed(allocator, leaf_layout)
            .expect("shared leaf allocation should succeed");
        write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
    }

    shared
        .allocate_bytes(allocator, array_layout, &payload)
        .expect("shared reference array allocation should succeed")
}

/// Build one local heap with an object graph ready to fork.
pub(crate) fn local_object_graph() -> (Heap, Vec<HeapReference>) {
    let mut heap = local_heap();
    let records = allocate_local_object_graph(&mut heap);

    (heap, records)
}

/// Build one deterministic byte source for access benchmarks.
pub(crate) fn source_bytes() -> Vec<u8> {
    let mut bytes = vec![0u8; ACCESS_BYTES];

    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = index as u8 ^ 0xA5;
    }

    bytes
}

/// Build one local heap with one access-sized payload.
pub(crate) fn local_access_allocation() -> (Heap, HeapReference) {
    let reference_map = ReferenceMap::None;
    let layout = AllocationLayout::new(ACCESS_BYTES, &reference_map);
    let payload = source_bytes();
    let mut heap = local_heap();
    let reference = heap
        .allocate_bytes(layout, &payload)
        .expect("access allocation should succeed");

    (heap, reference)
}

/// Build one shared heap with one access-sized payload.
pub(crate) fn shared_access_allocation() -> (SharedHeap, SharedHeapReference) {
    let reference_map = ReferenceMap::None;
    let layout = AllocationLayout::new(ACCESS_BYTES, &reference_map);
    let payload = source_bytes();
    let mut fixture = shared_fixture();
    let reference = fixture
        .heap
        .allocate_bytes(&mut fixture.allocator, layout, &payload)
        .expect("shared access allocation should succeed");

    (fixture.heap, reference)
}
