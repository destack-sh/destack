use crate::{
    HeapOptions, HeapSpace, Payload, RawSpace, SizeClassTable, test_allocator, test_layout,
};
use destack_mir::ReferenceMap;

/// Keep empty raw spans in the local cache instead of the live image.
#[test]
fn test_release_empty_raw_span_into_page_run_cache() {
    let options = HeapOptions {
        page_bytes: 16,
        raw_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let mut raw =
        RawSpace::with_options(allocator, &options).expect("explicit raw options should build");
    let pointer = raw
        .allocate(4, Payload::Bytes(&[1, 2, 3, 4]))
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
    let options = HeapOptions {
        page_bytes: 16,
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (layouts, layout_id) = test_layout(9, ReferenceMap::empty());
    let mut heap = HeapSpace::with_layouts_and_options(allocator, layouts, &options)
        .expect("explicit heap options should build");
    let reference = heap
        .allocate(layout_id, Payload::Bytes(&[9; 9]))
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
