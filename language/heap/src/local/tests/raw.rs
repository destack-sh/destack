use crate::{
    HeapError, HeapOptions, Payload, RawPointer, RawSpace, SizeClassTable, test_allocator,
};

/// Reclaim one freed raw allocation and keep the allocator live.
#[test]
fn test_free_raw_reclaims_live_allocation() {
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");
    let pointer = raw
        .allocate(2, Payload::Bytes(&[0xAB, 0xCD]))
        .expect("raw allocation should succeed");

    // freeing one live allocation should retire it immediately
    assert!(raw.is_live(pointer));
    assert!(raw.free(pointer).expect("raw free should succeed"));
    assert!(!raw.is_live(pointer));
}

/// Reclaim one freed raw large allocation and allow another large allocation.
#[test]
fn test_free_raw_reclaims_large_allocation() {
    let options = HeapOptions {
        heap_small_bytes: 32,
        raw_small_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let large_byte_len = options
        .size_classes
        .max_small_allocation_bytes()
        .expect("size class table should not be empty")
        + 1;
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");
    let pointer = raw
        .allocate(large_byte_len, Payload::Bytes(&vec![0xAB; large_byte_len]))
        .expect("raw large allocation should succeed");

    // freeing one large allocation should retire its pointer
    assert!(raw.free(pointer).expect("raw large free should succeed"));
    assert!(!raw.is_live(pointer));

    let next_pointer = raw
        .allocate(large_byte_len, Payload::Bytes(&vec![0xCD; large_byte_len]))
        .expect("raw large allocation should succeed");

    assert!(raw.is_live(next_pointer));
}

/// Reject one invalid raw pointer loudly.
#[test]
fn test_free_raw_rejects_invalid_pointer() {
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");

    // reject one unknown address
    let pointer = RawPointer::new(7);
    let error = raw.free(pointer).expect_err("raw free should fail");

    assert_eq!(error, HeapError::InvalidRawPointer { pointer });
}
