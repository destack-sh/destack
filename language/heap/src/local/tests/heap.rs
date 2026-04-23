use crate::local::space::HeapStorage;
use crate::{HeapError, HeapOptions, HeapReference, HeapSpace, test_allocator};
use destack_mir::LayoutTrace;

/// Reclaim one freed heap allocation and allow another allocation.
#[test]
fn test_free_heap_reclaims_live_allocation() {
    let options = HeapOptions::local();
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let reference = heap
        .allocate_bytes(&[0xAB], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");

    assert!(heap.is_live(reference));
    assert!(heap.free(reference).expect("heap free should succeed"));
    assert!(!heap.is_live(reference));

    let next_reference = heap
        .allocate_bytes(&[0xCD], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");

    assert!(heap.is_live(next_reference));
}

/// Reclaim one freed heap large allocation and allow another large allocation.
#[test]
fn test_free_heap_reclaims_large_allocation() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        ..HeapOptions::local()
    };
    let large_byte_len = options.size_classes.max_small_allocation_bytes() + 1;
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let first = heap
        .allocate_bytes(&vec![0xAB; large_byte_len], LayoutTrace::empty(), None)
        .expect("heap large allocation should succeed");

    assert!(matches!(heap.location(first), Some(HeapStorage::Large(_))));

    heap.free(first).expect("heap large free should succeed");

    let second = heap
        .allocate_bytes(&vec![0xCD; large_byte_len], LayoutTrace::empty(), None)
        .expect("heap large reallocation should succeed");

    assert!(matches!(heap.location(second), Some(HeapStorage::Large(_))));
}

/// Reject one invalid heap reference loudly.
#[test]
fn test_free_heap_rejects_invalid_reference() {
    let options = HeapOptions::local();
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let reference = HeapReference::new(7);
    let error = heap.free(reference).expect_err("heap free should fail");

    assert_eq!(error, HeapError::InvalidHeapReference { reference });
}
