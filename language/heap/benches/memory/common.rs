use std::sync::Arc;

use destack_heap::{
    AddressSpace, AllocationShape, Allocator, DEFAULT_PAGE_BYTES, Heap, HeapLimits, HeapOptions,
    HeapReference, SharedAllocator, SharedGcWorker, SharedHeap, SharedHeapLimits,
    SharedHeapReference,
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
/// The native pointer width used by heap references.
pub(crate) const REFERENCE_BYTES: usize = std::mem::size_of::<usize>();
/// The maximum number of allocations in one size-class matrix sample.
pub(crate) const MATRIX_MAX_ALLOCATIONS: usize = 8 * 1024;
/// The minimum number of allocations in one size-class matrix sample.
pub(crate) const MATRIX_MIN_ALLOCATIONS: usize = 128;
/// The target byte volume in one size-class matrix sample.
pub(crate) const MATRIX_SAMPLE_BYTES: usize = 1024 * 1024;
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

/// One shared heap with one worker-local allocator.
pub(crate) struct SharedFixture {
    /// The shared heap under test.
    pub(crate) heap: SharedHeap,
    /// The worker-local allocator cache.
    pub(crate) allocator: SharedAllocator,
    /// The shared collector worker used by this fixture.
    pub(crate) worker: SharedGcWorker,
}

/// Reserve one address space for memory benchmarks.
pub(crate) fn reserve_space() -> AddressSpace {
    AddressSpace::reserve(SPACE_BYTES, PAGE_BYTES).expect("address space should reserve")
}

/// Reserve one address space and materialize the requested page count.
pub(crate) fn materialized_space(page_count: usize) -> AddressSpace {
    let space = reserve_space();
    let page = vec![0xAB; PAGE_BYTES];

    // materialize exactly the pages requested by the benchmark case
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

    // give each sample its own allocator state
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

    // build the shared heap around one allocator
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

    // attach one worker-local allocator cache
    let allocator = shared.allocator();
    let worker = shared.register_collector_worker();

    SharedFixture {
        heap: shared,
        allocator,
        worker,
    }
}

/// Encode one machine word into one payload buffer.
pub(crate) fn write_word(bytes: &mut [u8], offset: usize, value: usize) {
    bytes[offset..offset + REFERENCE_BYTES].copy_from_slice(&value.to_le_bytes());
}

/// Build the scan map for one record with a local reference field.
pub(crate) fn local_record_reference_map() -> ReferenceMap {
    ReferenceMap::Direct {
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
    }
}

/// Build the scan map for one record with a shared reference field.
pub(crate) fn shared_record_reference_map() -> ReferenceMap {
    ReferenceMap::Direct {
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
    }
}

/// Build the scan map for one local reference array.
pub(crate) fn local_reference_array_map() -> ReferenceMap {
    ReferenceMap::Repeat {
        count: WORKLOAD_OBJECTS as u32,
        stride: REFERENCE_BYTES as u32,
        element: Box::new(local_record_reference_map()),
    }
}

/// Build the scan map for one shared reference array.
pub(crate) fn shared_reference_array_map() -> ReferenceMap {
    ReferenceMap::Repeat {
        count: WORKLOAD_OBJECTS as u32,
        stride: REFERENCE_BYTES as u32,
        element: Box::new(shared_record_reference_map()),
    }
}

/// Allocate one local object graph shaped like records pointing at leaf records.
pub(crate) fn allocate_local_object_graph(heap: &mut Heap) -> Vec<HeapReference> {
    // prepare scan maps and resolved layouts once
    let leaf_map = ReferenceMap::None;
    let record_map = local_record_reference_map();
    let leaf_shape = AllocationShape::new(LEAF_BYTES, 1, &leaf_map);
    let record_shape = AllocationShape::new(RECORD_BYTES, REFERENCE_BYTES, &record_map);
    let leaf_layout = heap.allocation_layout(leaf_shape);
    let record_layout = heap.allocation_layout(record_shape);
    let mut records = Vec::with_capacity(WORKLOAD_OBJECTS);

    // allocate leaf and record pairs
    for index in 0..WORKLOAD_OBJECTS {
        let leaf = heap
            .allocate_zeroed(&leaf_layout)
            .expect("leaf allocation should succeed");
        let mut record = [0u8; RECORD_BYTES];
        write_word(&mut record, 0, leaf.bits());
        write_word(&mut record, REFERENCE_BYTES, index);

        // copy the initialized record payload into managed memory
        let reference = heap
            .allocate_bytes(&record_layout, &record)
            .expect("record allocation should succeed");
        records.push(reference);
    }

    records
}

/// Allocate one shared object graph shaped like records pointing at leaf records.
pub(crate) fn allocate_shared_object_graph(
    shared: &SharedHeap,
    worker: &SharedGcWorker,
    allocator: &mut SharedAllocator,
) -> Vec<SharedHeapReference> {
    // prepare scan maps and resolved layouts once
    let leaf_map = ReferenceMap::None;
    let record_map = shared_record_reference_map();
    let leaf_shape = AllocationShape::new(LEAF_BYTES, 1, &leaf_map);
    let record_shape = AllocationShape::new(RECORD_BYTES, REFERENCE_BYTES, &record_map);
    let leaf_layout = shared.allocation_layout(leaf_shape);
    let record_layout = shared.allocation_layout(record_shape);
    let mut records = Vec::with_capacity(WORKLOAD_OBJECTS);

    // allocate leaf and record pairs through one worker cache
    for index in 0..WORKLOAD_OBJECTS {
        let leaf = shared
            .allocate_zeroed(worker, allocator, &leaf_layout)
            .expect("shared leaf allocation should succeed");
        let mut record = [0u8; RECORD_BYTES];
        write_word(&mut record, 0, leaf.bits());
        write_word(&mut record, REFERENCE_BYTES, index);

        // copy the initialized record payload into managed memory
        let reference = shared
            .allocate_bytes(worker, allocator, &record_layout, &record)
            .expect("shared record allocation should succeed");
        records.push(reference);
    }

    records
}

/// Allocate one local reference array backed by a repeated reference map.
pub(crate) fn allocate_local_reference_array(heap: &mut Heap) -> HeapReference {
    // prepare the leaf layout and repeated reference layout
    let leaf_map = ReferenceMap::None;
    let array_map = local_reference_array_map();
    let leaf_shape = AllocationShape::new(LEAF_BYTES, 1, &leaf_map);
    let array_shape = AllocationShape::new(
        WORKLOAD_OBJECTS * REFERENCE_BYTES,
        REFERENCE_BYTES,
        &array_map,
    );
    let leaf_layout = heap.allocation_layout(leaf_shape);
    let array_layout = heap.allocation_layout(array_shape);
    let mut payload = vec![0u8; WORKLOAD_OBJECTS * REFERENCE_BYTES];

    // build the array payload from fresh leaf references
    for index in 0..WORKLOAD_OBJECTS {
        let leaf = heap
            .allocate_zeroed(&leaf_layout)
            .expect("leaf allocation should succeed");
        write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
    }

    // copy the completed repeated-reference payload into the heap
    heap.allocate_bytes(&array_layout, &payload)
        .expect("reference array allocation should succeed")
}

/// Allocate one shared reference array backed by a repeated reference map.
pub(crate) fn allocate_shared_reference_array(
    shared: &SharedHeap,
    worker: &SharedGcWorker,
    allocator: &mut SharedAllocator,
) -> SharedHeapReference {
    // prepare the leaf layout and repeated reference layout
    let leaf_map = ReferenceMap::None;
    let array_map = shared_reference_array_map();
    let leaf_shape = AllocationShape::new(LEAF_BYTES, 1, &leaf_map);
    let array_shape = AllocationShape::new(
        WORKLOAD_OBJECTS * REFERENCE_BYTES,
        REFERENCE_BYTES,
        &array_map,
    );
    let leaf_layout = shared.allocation_layout(leaf_shape);
    let array_layout = shared.allocation_layout(array_shape);
    let mut payload = vec![0u8; WORKLOAD_OBJECTS * REFERENCE_BYTES];

    // build the array payload from fresh leaf references
    for index in 0..WORKLOAD_OBJECTS {
        let leaf = shared
            .allocate_zeroed(worker, allocator, &leaf_layout)
            .expect("shared leaf allocation should succeed");
        write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
    }

    // copy the completed repeated-reference payload into the heap
    shared
        .allocate_bytes(worker, allocator, &array_layout, &payload)
        .expect("shared reference array allocation should succeed")
}

/// Build one local heap with an object graph ready to fork.
pub(crate) fn local_object_graph() -> (Heap, Vec<HeapReference>) {
    let mut heap = local_heap();
    let records = allocate_local_object_graph(&mut heap);

    (heap, records)
}
