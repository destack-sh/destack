use crate::{HeapError, SharedRawPointer, SharedRawSpace};

/// Shared-memory free reports invalid pointers loudly.
#[test]
fn test_free_shared_rejects_invalid_pointer() {
    let mut shared = SharedRawSpace::new();

    // reject an unknown stable id
    let pointer = SharedRawPointer::new(7);
    let error = shared.free(pointer).expect_err("shared free should fail");

    assert_eq!(error, HeapError::InvalidSharedRawPointer { pointer });
}

/// Preserve stable shared ids across allocation growth and id reuse.
#[test]
fn test_allocate_shared_ids_reuse_after_free() {
    let mut shared = SharedRawSpace::new();
    let mut last = SharedRawPointer::NULL;

    // grow the shared entry table beyond one short run
    for index in 0..12 {
        last = shared
            .allocate_bytes(&[index as u8])
            .expect("shared allocation should succeed");
    }

    assert_eq!(last.id(), 12);

    // freed ids should be reused directly
    let reused = SharedRawPointer::new(7);
    assert!(shared.free(reused).expect("shared free should succeed"));

    let pointer = shared
        .allocate_bytes(&[0xEF])
        .expect("shared allocation should succeed");

    assert_eq!(pointer.id(), reused.id());
}

/// Shared-memory reads reject out-of-bounds pointer offsets loudly.
#[test]
fn test_shared_reads_reject_invalid_pointer_offset() {
    let mut shared = SharedRawSpace::new();
    let pointer = shared
        .allocate_bytes(&[0xAA, 0xBB, 0xCC])
        .expect("shared allocation should succeed");
    let pointer = SharedRawPointer::with_byte_offset(pointer.id(), 4);

    let error = shared
        .byte_len(pointer)
        .expect_err("shared byte_len should reject invalid offsets");

    assert_eq!(error, HeapError::InvalidSharedRawPointer { pointer });
}
