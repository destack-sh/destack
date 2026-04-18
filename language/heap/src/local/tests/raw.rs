use crate::tests::test_arena;
use crate::{HeapError, HeapOptions, RawPointer, RawSpace};

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
