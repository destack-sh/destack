use std::sync::Arc;

use crate::{Allocator, HeapError, Payload, SharedRawPointer, SharedRawSpace};

/// Reject one invalid shared raw pointer loudly.
#[test]
fn test_free_shared_rejects_invalid_pointer() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator);

    // reject one unknown address
    let pointer = SharedRawPointer::new(7);
    let error = shared.free(pointer).expect_err("shared free should fail");

    assert_eq!(error, HeapError::InvalidSharedRawPointer { pointer });
}

/// Reclaim one freed shared raw allocation and allow another allocation.
#[test]
fn test_free_shared_reclaims_live_allocation() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator);
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

/// Keep zero-byte shared raw allocations addressable.
#[test]
fn test_allocate_shared_zero_byte_raw_is_live() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let shared = SharedRawSpace::with_allocator(allocator);
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
    let shared = SharedRawSpace::with_allocator(allocator);
    let pointer = shared
        .allocate(3, Payload::Bytes(&[0xAA, 0xBB, 0xCC]))
        .expect("shared allocation should succeed");
    let pointer = pointer.add_bytes(4).expect("pointer offset should fit");

    let error = shared
        .byte_len(pointer)
        .expect_err("shared byte_len should reject invalid offsets");

    assert_eq!(error, HeapError::InvalidSharedRawPointer { pointer });
}
