use crate::{
    HeapOptions, HeapSpace, Payload, RawAllocationShape, RawSpace, SizeClassTable, test_allocator,
    test_layout,
};
use destack_mir::TraceMap;

/// The allocator chunk size for small-page cache fixtures.
const TEST_ALLOCATOR_CHUNK_BYTES: usize = 1024 * 1024;

/// Keep empty raw spans in the local cache instead of the live image.
#[test]
fn test_release_empty_raw_span_into_page_run_cache() {
    // build a raw space whose small spans are easy to observe
    let options = HeapOptions {
        page_bytes: 16,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        raw_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let mut raw =
        RawSpace::with_options(allocator, &options).expect("explicit raw options should build");
    let pointer = raw
        .allocate(RawAllocationShape::bytes(4), Payload::Bytes(&[1, 2, 3, 4]))
        .expect("raw allocation should succeed");

    // one live span should charge one full span of active bytes
    assert_eq!(raw.retained_bytes(), 32);

    // free the only allocation before capturing the raw image
    raw.free(pointer).expect("raw free should succeed");
    let image = raw.image().expect("raw image should capture");

    // capture boundaries should flush cached runs back into the allocator
    assert_eq!(raw.allocation_count(), 0);
    assert_eq!(raw.retained_bytes(), 0);
    assert!(image.spans()[0].bytes.is_empty());
}

/// Keep freed heap large-allocation pages in the local cache instead of the live image.
#[test]
fn test_release_heap_large_pages_into_page_run_cache() {
    // force the payload onto the large-allocation path
    let options = HeapOptions {
        page_bytes: 16,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let layout = test_layout(9, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[9; 9]),
        )
        .expect("heap allocation should succeed");

    // one live large allocation should charge one page of active bytes
    assert_eq!(heap.retained_bytes(), 16);

    // free the only allocation before capturing the heap image
    heap.free(reference).expect("heap free should succeed");
    let image = heap.image().expect("heap image should capture");

    // capture boundaries should flush cached runs back into the allocator
    assert_eq!(heap.allocation_count(), 0);
    assert_eq!(heap.allocated_bytes(), 0);
    assert_eq!(heap.retained_bytes(), 0);
    assert!(image.allocations()[0].pages.is_empty());
}
