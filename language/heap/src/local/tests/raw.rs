use crate::{
    HeapAllocationError, HeapError, HeapOptions, Payload, RawAllocationShape, RawPointer, RawSpace,
    SizeClassTable, test_allocator,
};

/// Reclaim one freed raw allocation and keep the allocator live.
#[test]
fn test_free_raw_reclaims_live_allocation() {
    // allocate one live raw payload
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");
    let pointer = raw
        .allocate(RawAllocationShape::bytes(2), Payload::Bytes(&[0xAB, 0xCD]))
        .expect("raw allocation should succeed");

    // freeing one live allocation should retire it immediately
    assert!(raw.is_live(pointer));
    raw.free(pointer).expect("raw free should succeed");
    assert!(!raw.is_live(pointer));
}

/// Clear bytes when reusing one freed raw slot for zeroed allocation.
#[test]
fn test_allocate_zeroed_raw_clears_reused_slot() {
    // seed one nonzero raw slot
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");
    let pointer = raw
        .allocate(RawAllocationShape::bytes(2), Payload::Bytes(&[0xAB, 0xCD]))
        .expect("raw allocation should succeed");

    // reuse the freed slot with zeroed payload
    raw.free(pointer).expect("raw free should succeed");
    let pointer = raw
        .allocate(RawAllocationShape::bytes(2), Payload::Zeroed)
        .expect("zeroed raw allocation should succeed");

    assert_eq!(raw.read_bytes(pointer), Ok(vec![0, 0]));
}

/// Reject caller-provided raw bytes that do not match the allocation shape.
#[test]
fn test_reject_raw_allocation_byte_len_mismatch() {
    // build a default raw space
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");

    // provide fewer bytes than the requested shape
    let error = raw
        .allocate(RawAllocationShape::new(4, 1), Payload::Bytes(&[1, 2]))
        .expect_err("raw allocation should reject mismatched bytes");

    assert_eq!(
        error,
        HeapError::invalid_allocation(HeapAllocationError::ByteLengthMismatch {
            expected: 4,
            actual: 2
        })
    );
}

/// Honor the requested raw allocation base alignment.
#[test]
fn test_allocate_raw_honors_alignment() {
    // build a default raw space
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");

    // align both small slots and page-backed allocations
    let small = raw
        .allocate(RawAllocationShape::new(1, 16), Payload::Zeroed)
        .expect("aligned small raw allocation should succeed");
    let large = raw
        .allocate(
            RawAllocationShape::new(
                options.raw_small_size_bytes + 1,
                options.page_size_bytes * 2,
            ),
            Payload::Zeroed,
        )
        .expect("aligned large raw allocation should succeed");

    assert_eq!(small.offset() % 16, 0);
    assert_eq!(large.offset() % (options.page_size_bytes * 2), 0);
}

/// Keep raw fork writes independent from the parent mapping.
#[test]
fn test_fork_raw_write_is_independent() {
    // allocate one raw payload before forking
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");
    let pointer = raw
        .allocate(RawAllocationShape::bytes(4), Payload::Bytes(&[1, 2, 3, 4]))
        .expect("raw allocation should succeed");
    let mut forked = raw.fork().expect("raw fork should succeed");

    // mutate the fork through the same logical pointer
    forked
        .write_bytes(pointer, 1, &[9, 8])
        .expect("forked raw write should succeed");

    assert_eq!(raw.read_bytes(pointer), Ok(vec![1, 2, 3, 4]));
    assert_eq!(forked.read_bytes(pointer), Ok(vec![1, 9, 8, 4]));
}

/// Reclaim one freed raw large allocation and allow another large allocation.
#[test]
fn test_free_raw_reclaims_large_allocation() {
    // force allocations larger than the local small span classes
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        raw_small_size_bytes: 32,
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
        .allocate(
            RawAllocationShape::bytes(large_byte_len),
            Payload::Bytes(&vec![0xAB; large_byte_len]),
        )
        .expect("raw large allocation should succeed");

    // freeing one large allocation should retire its pointer
    raw.free(pointer).expect("raw large free should succeed");
    assert!(!raw.is_live(pointer));

    // allocating again should keep the raw space usable
    let next_pointer = raw
        .allocate(
            RawAllocationShape::bytes(large_byte_len),
            Payload::Bytes(&vec![0xCD; large_byte_len]),
        )
        .expect("raw large allocation should succeed");

    assert!(raw.is_live(next_pointer));
}

/// Re-place one raw allocation when replacement bytes fit a smaller class.
#[test]
fn test_replace_large_raw_can_move_to_small() {
    // allocate one payload above the local small raw threshold
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        raw_small_size_bytes: 32,
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
        .allocate(
            RawAllocationShape::bytes(large_byte_len),
            Payload::Bytes(&vec![0xAB; large_byte_len]),
        )
        .expect("raw large allocation should succeed");

    // shrinking a raw allocation should use the normal placement path
    let pointer = raw
        .replace_bytes(pointer, &[0xCD])
        .expect("raw replacement should succeed");

    assert_eq!(raw.read_bytes(pointer), Ok(vec![0xCD]));
    assert_eq!(raw.byte_len(pointer), Ok(1));
}

/// Reject one invalid raw pointer loudly.
#[test]
fn test_free_raw_rejects_invalid_pointer() {
    // build a raw space with no allocation at offset 7
    let options = HeapOptions::local();
    let mut raw =
        RawSpace::with_options(test_allocator(&options), &options).expect("raw space should build");

    // reject one unknown address
    let pointer = RawPointer::new(7);
    let error = raw.free(pointer).expect_err("raw free should fail");

    assert_eq!(error, HeapError::invalid_raw_pointer(pointer));
}
