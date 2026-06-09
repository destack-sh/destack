use crate::local::storage::HeapStorage;
use crate::{HeapOptions, Payload, SizeClassTable, test_allocator, test_layout};
use destack_mir::TraceMap;

use super::heap_allocation_plan;

/// The allocator chunk size for small-page cache fixtures.
const TEST_ALLOCATOR_CHUNK_SIZE_BYTES: usize = 1024 * 1024;

/// Keep freed heap large-block pages in the local cache instead of the live image.
#[test]
fn test_release_heap_large_pages_into_page_span_cache() {
    // force the payload onto the large-block path
    let options = HeapOptions {
        page_size_bytes: 16,
        allocator_chunk_size_bytes: TEST_ALLOCATOR_CHUNK_SIZE_BYTES,
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let layout = test_layout(9, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(allocator, &options)
        .expect("explicit heap options should build");
    let reference = heap
        .allocate(
            &heap_allocation_plan(&heap, layout.block()),
            Payload::Bytes(&[9; 9]),
        )
        .expect("heap block should succeed");

    // one live large block should charge one page of active bytes
    assert_eq!(heap.retained_bytes(), 16);

    // free the only block before capturing the heap image
    heap.free(reference).expect("heap free should succeed");
    let image = heap.image().expect("heap image should capture");

    // capture boundaries should flush cached spans back into the allocator
    assert_eq!(heap.allocation_count(), 0);
    assert_eq!(heap.allocated_bytes(), 0);
    assert_eq!(heap.retained_bytes(), 0);
    assert!(image.blocks()[0].bytes.is_empty());
}
