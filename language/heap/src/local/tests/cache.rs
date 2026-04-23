use crate::{HeapOptions, HeapSpace, RawSpace, SizeClassTable, test_allocator};
use destack_mir::LayoutTrace;

/// Keep empty raw spans in the local cache instead of the live image.
#[test]
fn test_release_empty_raw_span_into_page_run_cache() {
    let layout = HeapOptions {
        page_bytes: 16,
        raw_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&layout);
    let mut raw =
        RawSpace::with_options(allocator, &layout).expect("explicit raw layout should build");
    let pointer = raw
        .allocate_bytes(&[1, 2, 3, 4])
        .expect("raw allocation should succeed");

    // one live span should charge one full span of active bytes
    assert_eq!(raw.active_bytes(), 32);

    raw.free(pointer).expect("raw free should succeed");
    let image = raw.image();

    // capture boundaries should flush cached runs back into the allocator
    assert_eq!(raw.allocation_count(), 0);
    assert_eq!(raw.active_bytes(), 0);
    assert!(image.spans()[0].bytes.is_empty());
}

/// Keep freed heap large-entry pages in the local cache instead of the live image.
#[test]
fn test_release_heap_large_pages_into_page_run_cache() {
    let layout = HeapOptions {
        page_bytes: 16,
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap layout should build");
    let reference = heap
        .allocate_bytes(&[9; 9], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");

    // one live large entry should charge one page of active bytes
    assert_eq!(heap.active_bytes(), 16);

    heap.free(reference).expect("heap free should succeed");
    let image = heap.image().expect("heap image should capture");

    // capture boundaries should flush cached runs back into the allocator
    assert_eq!(heap.allocation_count(), 0);
    assert_eq!(heap.allocated_bytes(), 0);
    assert_eq!(heap.active_bytes(), 0);
    assert!(image.entries()[0].pages.is_empty());
}
