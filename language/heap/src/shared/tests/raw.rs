use std::sync::Arc;

use crate::{
    Allocator, HeapAllocationError, HeapError, Payload, RawAllocationShape, SharedRawPointer,
    SharedRawSpace,
};

/// Build one default shared raw space.
fn test_shared_raw_space() -> SharedRawSpace {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));

    SharedRawSpace::with_allocator(allocator).expect("shared raw should build")
}

/// Reject one invalid shared raw pointer loudly.
#[test]
fn test_free_shared_rejects_invalid_pointer() {
    // build a shared raw space with no block at offset 7
    let shared = test_shared_raw_space();

    // reject one unknown address
    let pointer = SharedRawPointer::new(7);
    let error = shared.free(pointer).expect_err("shared free should fail");

    assert_eq!(error, HeapError::invalid_shared_raw_pointer(pointer));
}

/// Reclaim one freed shared raw block and allow another block.
#[test]
fn test_free_shared_reclaims_live_allocation() {
    // allocate one live shared raw payload
    let shared = test_shared_raw_space();
    let pointer = shared
        .allocate(RawAllocationShape::bytes(2), Payload::Bytes(&[0xAB, 0xCD]))
        .expect("shared block should succeed");

    // freeing one live block should retire it immediately
    assert!(shared.is_live(pointer));
    shared.free(pointer).expect("shared free should succeed");
    assert!(!shared.is_live(pointer));

    // allocate again to prove the shared raw space remains usable
    let next_pointer = shared
        .allocate(RawAllocationShape::bytes(1), Payload::Bytes(&[0xEF]))
        .expect("shared block should succeed");

    assert!(shared.is_live(next_pointer));
}

/// Clear bytes when reusing one freed shared raw block for zeroed payload.
#[test]
fn test_allocate_zeroed_shared_raw_clears_reused_allocation() {
    // seed one nonzero shared raw block
    let shared = test_shared_raw_space();
    let pointer = shared
        .allocate(RawAllocationShape::bytes(2), Payload::Bytes(&[0xAB, 0xCD]))
        .expect("shared block should succeed");

    // reuse the freed block with zeroed payload
    shared.free(pointer).expect("shared free should succeed");
    let pointer = shared
        .allocate(RawAllocationShape::bytes(2), Payload::Zeroed)
        .expect("zeroed shared block should succeed");

    assert_eq!(shared.read_bytes(pointer), Ok(vec![0, 0]));
}

/// Reject caller-provided shared raw bytes that do not match the allocation shape.
#[test]
fn test_reject_shared_raw_allocation_byte_len_mismatch() {
    // build a default shared raw space
    let shared = test_shared_raw_space();

    // provide fewer bytes than the requested shape
    let error = shared
        .allocate(RawAllocationShape::new(4, 1), Payload::Bytes(&[1, 2]))
        .expect_err("shared block should reject mismatched bytes");

    assert_eq!(
        error,
        HeapError::invalid_allocation(HeapAllocationError::ByteLengthMismatch {
            expected: 4,
            actual: 2
        })
    );
}

/// Honor the requested shared raw block base alignment.
#[test]
fn test_allocate_shared_raw_honors_alignment() {
    // build a default shared raw space
    let shared = test_shared_raw_space();
    let alignment = shared.page_size_bytes() * 2;

    // align page-backed shared raw block bases
    let pointer = shared
        .allocate(RawAllocationShape::new(1, alignment), Payload::Zeroed)
        .expect("aligned shared raw block should succeed");

    assert_eq!(pointer.offset() % alignment, 0);
}

/// Keep shared raw fork writes independent from the parent mapping.
#[test]
fn test_fork_shared_raw_write_is_independent() {
    // allocate one shared raw payload before forking
    let shared = test_shared_raw_space();
    let pointer = shared
        .allocate(RawAllocationShape::bytes(4), Payload::Bytes(&[1, 2, 3, 4]))
        .expect("shared block should succeed");
    let forked = shared.fork().expect("shared raw fork should succeed");

    // mutate the fork through the same logical pointer
    forked
        .write_bytes(pointer, 1, &[9, 8])
        .expect("forked shared raw write should succeed");

    assert_eq!(shared.read_bytes(pointer), Ok(vec![1, 2, 3, 4]));
    assert_eq!(forked.read_bytes(pointer), Ok(vec![1, 9, 8, 4]));
}

/// Keep zero-byte shared raw blocks addressable.
#[test]
fn test_allocate_shared_zero_byte_raw_is_live() {
    // allocate a zero-byte payload as a real resource
    let shared = test_shared_raw_space();
    let pointer = shared
        .allocate(RawAllocationShape::bytes(0), Payload::Bytes(&[]))
        .expect("zero-byte block should succeed");

    assert!(shared.is_live(pointer));
    assert_eq!(shared.byte_len(pointer), Ok(0));
}

/// Reject one out of bounds shared pointer offset loudly.
#[test]
fn test_shared_reads_reject_invalid_pointer_offset() {
    // allocate one payload and form an offset outside it
    let shared = test_shared_raw_space();
    let pointer = shared
        .allocate(
            RawAllocationShape::bytes(3),
            Payload::Bytes(&[0xAA, 0xBB, 0xCC]),
        )
        .expect("shared block should succeed");
    let pointer = pointer.add_bytes(4);

    // reject the interior pointer because it resolves outside the block
    let error = shared
        .byte_len(pointer)
        .expect_err("shared byte_len should reject invalid offsets");

    assert_eq!(error, HeapError::invalid_shared_raw_pointer(pointer));
}
