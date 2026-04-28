use crate::local::space::HeapPlace;
use crate::{
    HeapError, HeapOptions, HeapReference, HeapSpace, Payload, SizeClassTable, test_allocator,
    test_layout,
};
use destack_mir::ReferenceMap;

/// Reject one zero-size managed heap allocation.
#[test]
fn test_allocate_heap_rejects_zero_size_layout() {
    let options = HeapOptions::local();
    let layout = test_layout(0, ReferenceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let error = heap
        .allocate(layout.allocation(), Payload::Bytes(&[]))
        .expect_err("heap allocation should reject zero-size layouts");

    assert_eq!(error, HeapError::ZeroSizeAllocation);
}

/// Reclaim one freed heap allocation and allow another allocation.
#[test]
fn test_free_heap_reclaims_live_allocation() {
    let options = HeapOptions::local();
    let layout = test_layout(1, ReferenceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let reference = heap
        .allocate(layout.allocation(), Payload::Bytes(&[0xAB]))
        .expect("heap allocation should succeed");

    assert!(heap.is_live(reference));
    assert!(heap.free(reference).expect("heap free should succeed"));
    assert!(!heap.is_live(reference));

    let next_reference = heap
        .allocate(layout.allocation(), Payload::Bytes(&[0xCD]))
        .expect("heap allocation should succeed");

    assert!(heap.is_live(next_reference));
}

/// Clear one reused small heap slot before writing a shorter payload.
#[test]
fn test_allocate_heap_clears_reused_small_slot_tail() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let full_layout = test_layout(8, ReferenceMap::empty());
    let short_layout = test_layout(1, ReferenceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let first = heap
        .allocate(full_layout.allocation(), Payload::Bytes(&[0xAA; 8]))
        .expect("heap allocation should succeed");
    let second = heap
        .allocate(short_layout.allocation(), Payload::Bytes(&[0xBB]))
        .expect("heap allocation should succeed");

    assert!(heap.free(first).expect("heap free should succeed"));

    let reused = heap
        .allocate(short_layout.allocation(), Payload::Bytes(&[0xCC]))
        .expect("heap allocation should succeed");

    assert!(heap.is_live(second));
    assert_eq!(heap.read_bytes(reused), Ok(vec![0xCC, 0, 0, 0, 0, 0, 0, 0]));
}

/// Reject one write that crosses allocation bounds from an interior reference.
#[test]
fn test_write_heap_rejects_interior_reference_crossing_bounds() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 8,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_layout(1, ReferenceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let reference = heap
        .allocate(layout.allocation(), Payload::Bytes(&[0xAB]))
        .expect("heap allocation should succeed");
    let reference = reference.add_bytes(7);

    let error = heap
        .write_bytes(reference, 0, &[1, 2])
        .expect_err("heap write should reject bounds crossing");

    assert_eq!(
        error,
        HeapError::InvalidByteRange {
            start: 7,
            len: 2,
            capacity: 8
        }
    );
}

/// Reclaim one freed heap large allocation and allow another large allocation.
#[test]
fn test_free_heap_reclaims_large_allocation() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let large_byte_len = options
        .size_classes
        .max_small_allocation_bytes()
        .expect("size class table should not be empty")
        + 1;
    let layout = test_layout(large_byte_len, ReferenceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let first = heap
        .allocate(
            layout.allocation(),
            Payload::Bytes(&vec![0xAB; large_byte_len]),
        )
        .expect("heap large allocation should succeed");

    assert!(matches!(heap.place(first), Some(HeapPlace::Large(_))));

    heap.free(first).expect("heap large free should succeed");

    let second = heap
        .allocate(
            layout.allocation(),
            Payload::Bytes(&vec![0xCD; large_byte_len]),
        )
        .expect("heap large reallocation should succeed");

    assert!(matches!(heap.place(second), Some(HeapPlace::Large(_))));
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
