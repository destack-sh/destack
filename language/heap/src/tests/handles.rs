use crate::{
    HeapError, HeapOptions, ManagedReference, ManagedSpace, RawPointer, RawSpace, ReferenceMap,
    SharedPointer, SharedSpace,
};

use super::tests::test_arena;

/// Preserve stable managed ids across allocation growth and id reuse.
#[test]
fn test_allocate_managed_ids_reuse_after_free() {
    let options = HeapOptions::default();
    let mut managed = ManagedSpace::with_options(test_arena(&options), &options)
        .expect("default managed options should build");
    let mut last = ManagedReference::NULL;

    // grow the reference table beyond one short run
    for index in 0..12 {
        last = managed
            .allocate_bytes(&[index as u8], ReferenceMap::empty(), None)
            .expect("managed allocation should succeed");
    }

    assert_eq!(last.id(), 12);

    // freed ids should be reused directly
    let reused = ManagedReference::new(7);
    assert!(managed.free(reused).expect("managed free should succeed"));

    let reference = managed
        .allocate_bytes(&[0xAB], ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");

    assert_eq!(reference.id(), reused.id());
}

/// Reuse freed managed large-entry ids on ordinary large frees.
#[test]
fn test_free_managed_reuses_large_entry_ids() {
    let options = HeapOptions {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        ..HeapOptions::default()
    };
    let large_byte_len = options.size_classes.max_small_allocation_bytes() + 1;
    let mut managed = ManagedSpace::with_options(test_arena(&options), &options)
        .expect("explicit managed options should build");
    let first = managed
        .allocate_bytes(&vec![0xAB; large_byte_len], ReferenceMap::empty(), None)
        .expect("managed large allocation should succeed");

    // the first large allocation should consume the first large-entry id
    assert!(matches!(
        managed.location(first),
        Some(crate::managed::ManagedLocation::Large(entry_id)) if entry_id.id() == 1
    ));
    assert_eq!(managed.large.free_large_entry_ids, Vec::<u64>::new());

    managed
        .free(first)
        .expect("managed large free should succeed");

    // ordinary frees should return the retired large-entry id immediately
    assert_eq!(managed.large.free_large_entry_ids, vec![1]);

    let second = managed
        .allocate_bytes(&vec![0xCD; large_byte_len], ReferenceMap::empty(), None)
        .expect("managed large reallocation should succeed");

    assert!(matches!(
        managed.location(second),
        Some(crate::managed::ManagedLocation::Large(entry_id)) if entry_id.id() == 1
    ));
    assert_eq!(managed.large.free_large_entry_ids, Vec::<u64>::new());
    assert_eq!(managed.large.next_unused_large_entry_id, 2);
}

/// Managed free reports invalid references loudly.
#[test]
fn test_free_managed_rejects_invalid_reference() {
    let options = HeapOptions::default();
    let mut managed = ManagedSpace::with_options(test_arena(&options), &options)
        .expect("default managed options should build");

    // reject an unknown stable id
    let reference = ManagedReference::new(7);
    let error = managed
        .free(reference)
        .expect_err("managed free should fail");

    assert_eq!(error, HeapError::InvalidManagedReference { reference });
}

/// Preserve stable raw ids across allocation growth and id reuse.
#[test]
fn test_allocate_raw_ids_reuse_after_free() {
    let options = HeapOptions::default();
    let mut raw = RawSpace::with_options(test_arena(&options), &options)
        .expect("default raw options should build");
    let mut last = RawPointer::NULL;

    // grow the pointer table beyond one short run
    for index in 0..12 {
        last = raw
            .allocate_bytes(&[index as u8])
            .expect("raw allocation should succeed");
    }

    assert_eq!(last.id(), 12);

    // freed ids should be reused directly
    let reused = RawPointer::new(7);
    assert!(raw.free(reused).expect("raw free should succeed"));

    let pointer = raw
        .allocate_bytes(&[0xCD])
        .expect("raw allocation should succeed");

    assert_eq!(pointer.id(), reused.id());
}

/// Reuse freed raw large-entry ids on ordinary large frees.
#[test]
fn test_free_raw_reuses_large_entry_ids() {
    let options = HeapOptions {
        managed_small_bytes: 32,
        raw_small_bytes: 32,
        ..HeapOptions::default()
    };
    let large_byte_len = options.size_classes.max_small_allocation_bytes() + 1;
    let mut raw = RawSpace::with_options(test_arena(&options), &options)
        .expect("explicit raw options should build");
    let first = raw
        .allocate_bytes(&vec![0xAB; large_byte_len])
        .expect("raw large allocation should succeed");

    // the first large allocation should consume the first large-entry id
    assert_eq!(raw.large.free_large_entry_ids, Vec::<u64>::new());
    assert_eq!(raw.large.next_unused_large_entry_id, 2);
    assert!(raw.large.entries.get(0).is_some_and(|entry| entry.is_live));

    raw.free(first).expect("raw large free should succeed");

    // ordinary frees should return the retired large-entry id immediately
    assert_eq!(raw.large.free_large_entry_ids, vec![1]);
    assert!(!raw.large.entries.get(0).is_some_and(|entry| entry.is_live));

    raw.allocate_bytes(&vec![0xCD; large_byte_len])
        .expect("raw large reallocation should succeed");

    assert_eq!(raw.large.free_large_entry_ids, Vec::<u64>::new());
    assert_eq!(raw.large.next_unused_large_entry_id, 2);
    assert!(raw.large.entries.get(0).is_some_and(|entry| entry.is_live));
}

/// Raw free reports invalid pointers loudly.
#[test]
fn test_free_raw_rejects_invalid_pointer() {
    let options = HeapOptions::default();
    let mut raw = RawSpace::with_options(test_arena(&options), &options)
        .expect("default raw options should build");

    // reject an unknown stable id
    let pointer = RawPointer::new(7);
    let error = raw.free(pointer).expect_err("raw free should fail");

    assert_eq!(error, HeapError::InvalidRawPointer { pointer });
}

/// Shared-memory free reports invalid pointers loudly.
#[test]
fn test_free_shared_rejects_invalid_pointer() {
    let mut shared = SharedSpace::new();

    // reject an unknown stable id
    let pointer = SharedPointer::new(7);
    let error = shared.free(pointer).expect_err("shared free should fail");

    assert_eq!(error, HeapError::InvalidSharedPointer { pointer });
}

/// Preserve stable shared ids across allocation growth and id reuse.
#[test]
fn test_allocate_shared_ids_reuse_after_free() {
    let mut shared = SharedSpace::new();
    let mut last = SharedPointer::NULL;

    // grow the shared entry table beyond one short run
    for index in 0..12 {
        last = shared
            .allocate_bytes(&[index as u8])
            .expect("shared allocation should succeed");
    }

    assert_eq!(last.id(), 12);

    // freed ids should be reused directly
    let reused = SharedPointer::new(7);
    assert!(shared.free(reused).expect("shared free should succeed"));

    let pointer = shared
        .allocate_bytes(&[0xEF])
        .expect("shared allocation should succeed");

    assert_eq!(pointer.id(), reused.id());
}

/// Shared-memory reads reject out-of-bounds pointer offsets loudly.
#[test]
fn test_shared_reads_reject_invalid_pointer_offset() {
    let mut shared = SharedSpace::new();
    let pointer = shared
        .allocate_bytes(&[0xAA, 0xBB, 0xCC])
        .expect("shared allocation should succeed");
    let pointer = SharedPointer::with_byte_offset(pointer.id(), 4);

    let error = shared
        .byte_len(pointer)
        .expect_err("shared byte_len should reject invalid offsets");

    assert_eq!(error, HeapError::InvalidSharedPointer { pointer });
}
