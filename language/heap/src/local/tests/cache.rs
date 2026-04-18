use crate::tests::test_arena;
use crate::{EdgeMap, HeapOptions, ManagedSpace, RawSpace, SizeClassTable};

/// Keep empty raw spans in the local cache instead of the live image.
#[test]
fn test_release_empty_raw_span_into_page_run_cache() {
    let layout = HeapOptions {
        page_bytes: 16,
        raw_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::default()
    };
    let arena = test_arena(&layout);
    let mut raw = RawSpace::with_options(arena, &layout).expect("explicit raw layout should build");
    let pointer = raw
        .allocate_bytes(&[1, 2, 3, 4])
        .expect("raw allocation should succeed");

    // one live span should charge one full span of active bytes
    assert_eq!(raw.active_bytes(), 32);

    raw.free(pointer).expect("raw free should succeed");
    let image = raw.image();

    // the freed span backing should stay cached, not live in the image
    assert_eq!(raw.allocation_count(), 0);
    assert_eq!(raw.active_bytes(), 32);
    assert!(image.spans()[0].pages.is_empty());
}

/// Keep freed managed large-entry pages in the local cache instead of the live image.
#[test]
fn test_release_managed_large_pages_into_page_run_cache() {
    let layout = HeapOptions {
        page_bytes: 16,
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::default()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed layout should build");
    let reference = managed
        .allocate_bytes(&[9; 9], EdgeMap::empty(), None)
        .expect("managed allocation should succeed");

    // one live large entry should charge one page of active bytes
    assert_eq!(managed.active_bytes(), 16);

    managed
        .free(reference)
        .expect("managed free should succeed");
    let image = managed.image().expect("managed image should capture");

    // the freed large-entry backing should stay cached, not live in the image
    assert_eq!(managed.allocation_count(), 0);
    assert_eq!(managed.allocated_bytes(), 0);
    assert_eq!(managed.active_bytes(), 16);
    assert!(image.entries()[0].pages.is_empty());
}
