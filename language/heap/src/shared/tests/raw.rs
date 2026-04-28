use std::sync::Arc;

use crate::{Allocator, HeapError, Payload, SharedRawPointer, SharedRawSpace};

/// Reject one invalid shared raw pointer loudly.
#[test]
fn test_free_shared_rejects_invalid_pointer() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator).expect("shared raw should build");

    // reject one unknown address
    let pointer = SharedRawPointer::new(7);
    let error = shared.free(pointer).expect_err("shared free should fail");

    assert_eq!(error, HeapError::InvalidSharedRawPointer { pointer });
}

/// Reclaim one freed shared raw allocation and allow another allocation.
#[test]
fn test_free_shared_reclaims_live_allocation() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator).expect("shared raw should build");
    let pointer = shared
        .allocate(2, Payload::Bytes(&[0xAB, 0xCD]))
        .expect("shared allocation should succeed");

    // freeing one live allocation should retire it immediately
    assert!(shared.is_live(pointer));
    assert!(shared.free(pointer).expect("shared free should succeed"));
    assert!(!shared.is_live(pointer));

    let next_pointer = shared
        .allocate(1, Payload::Bytes(&[0xEF]))
        .expect("shared allocation should succeed");

    assert!(shared.is_live(next_pointer));
}

/// Clear bytes when reusing one freed shared raw allocation for zeroed payload.
#[test]
fn test_allocate_zeroed_shared_raw_clears_reused_allocation() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator).expect("shared raw should build");
    let pointer = shared
        .allocate(2, Payload::Bytes(&[0xAB, 0xCD]))
        .expect("shared allocation should succeed");

    // reuse the freed allocation with zeroed payload
    assert!(shared.free(pointer).expect("shared free should succeed"));
    let pointer = shared
        .allocate(2, Payload::Zeroed)
        .expect("zeroed shared allocation should succeed");

    assert_eq!(shared.read_bytes(pointer), Ok(vec![0, 0]));
}

/// Keep shared raw fork writes independent from the parent mapping.
#[test]
fn test_fork_shared_raw_write_is_independent() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator).expect("shared raw should build");
    let pointer = shared
        .allocate(4, Payload::Bytes(&[1, 2, 3, 4]))
        .expect("shared allocation should succeed");
    let forked = shared.fork().expect("shared raw fork should succeed");

    // mutate the fork through the same logical pointer
    forked
        .write_bytes(pointer, 1, &[9, 8])
        .expect("forked shared raw write should succeed");

    assert_eq!(shared.read_bytes(pointer), Ok(vec![1, 2, 3, 4]));
    assert_eq!(forked.read_bytes(pointer), Ok(vec![1, 9, 8, 4]));
}

/// Keep zero-byte shared raw allocations addressable.
#[test]
fn test_allocate_shared_zero_byte_raw_is_live() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator).expect("shared raw should build");
    let pointer = shared
        .allocate(0, Payload::Bytes(&[]))
        .expect("zero-byte allocation should succeed");

    assert!(shared.is_live(pointer));
    assert_eq!(shared.byte_len(pointer), Ok(0));
}

/// Reject one out of bounds shared pointer offset loudly.
#[test]
fn test_shared_reads_reject_invalid_pointer_offset() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator).expect("shared raw should build");
    let pointer = shared
        .allocate(3, Payload::Bytes(&[0xAA, 0xBB, 0xCC]))
        .expect("shared allocation should succeed");
    let pointer = pointer.add_bytes(4);

    let error = shared
        .byte_len(pointer)
        .expect_err("shared byte_len should reject invalid offsets");

    assert_eq!(error, HeapError::InvalidSharedRawPointer { pointer });
}
