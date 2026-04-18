use crate::tests::test_arena;
use crate::{EdgeMap, HeapError, HeapOptions, ManagedReference, ManagedSpace};

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
            .allocate_bytes(&[index as u8], EdgeMap::empty(), None)
            .expect("managed allocation should succeed");
    }

    assert_eq!(last.id(), 12);

    // freed ids should be reused directly
    let reused = ManagedReference::new(7);
    assert!(managed.free(reused).expect("managed free should succeed"));

    let reference = managed
        .allocate_bytes(&[0xAB], EdgeMap::empty(), None)
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
        .allocate_bytes(&vec![0xAB; large_byte_len], EdgeMap::empty(), None)
        .expect("managed large allocation should succeed");

    // the first large allocation should consume the first large-entry id
    assert!(matches!(
        managed.location(first),
        Some(crate::local::managed::ManagedLocation::Large(entry_id)) if entry_id.id() == 1
    ));
    assert_eq!(managed.large.free_large_entry_ids, Vec::<u64>::new());

    managed
        .free(first)
        .expect("managed large free should succeed");

    // ordinary frees should return the retired large-entry id immediately
    assert_eq!(managed.large.free_large_entry_ids, vec![1]);

    let second = managed
        .allocate_bytes(&vec![0xCD; large_byte_len], EdgeMap::empty(), None)
        .expect("managed large reallocation should succeed");

    assert!(matches!(
        managed.location(second),
        Some(crate::local::managed::ManagedLocation::Large(entry_id)) if entry_id.id() == 1
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
