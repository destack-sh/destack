use crate::managed::ManagedLocation;
use crate::{
    GcCycle, GcKind, HeapError, HeapOptions, ManagedReference, ManagedSpace, ReferenceMap,
};

use super::tests::test_arena;

/// Promote reachable young entries and clear unreachable young-space state.
#[test]
fn test_collect_young_references_promotes_reachable_entries() {
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::default()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");

    // allocate one reachable and one unreachable young entry
    let reachable = managed
        .allocate_bytes(&[1, 2, 3], ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let unreachable = managed
        .allocate_bytes(&[4, 5, 6], ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");

    assert!(matches!(
        managed.location(reachable),
        Some(ManagedLocation::Young(_))
    ));
    assert!(matches!(
        managed.location(unreachable),
        Some(ManagedLocation::Young(_))
    ));

    // collect against the reachable root
    let stats = managed
        .collect_young_references([reachable])
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(managed.read_bytes(reachable), Ok(vec![1, 2, 3]));
    assert!(!managed.is_live(unreachable));
    assert!(matches!(
        managed.location(reachable),
        Some(ManagedLocation::Small(_)) | Some(ManagedLocation::Large(_))
    ));

    // the young space should reset to an empty cursor over a fresh page run
    let image = managed.image().expect("managed image should capture");
    let young = image.young();

    assert_eq!(young.next_offset(), 0);
    assert_eq!(
        young.pages().len(),
        young.capacity_bytes().div_ceil(young.page_bytes())
    );
}

/// Promote young entries reached through traced young references.
#[test]
fn test_collect_young_references_promotes_reachable_child_entries() {
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::default()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let reference_map = ReferenceMap::ReferenceOffsets {
        offsets: vec![0].into_boxed_slice(),
    };

    // build one young parent that points at one young child
    let child = managed
        .allocate_bytes(&[0xC1, 0x1D], ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let parent = managed
        .allocate_bytes(&child.bits().to_le_bytes(), reference_map, None)
        .expect("managed allocation should succeed");

    let stats = managed
        .collect_young_references([parent])
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(managed.read_bytes(child), Ok(vec![0xC1, 0x1D]));
    assert!(matches!(
        managed.location(parent),
        Some(ManagedLocation::Small(_)) | Some(ManagedLocation::Large(_))
    ));
    assert!(matches!(
        managed.location(child),
        Some(ManagedLocation::Small(_)) | Some(ManagedLocation::Large(_))
    ));
}

/// Reject invalid non-null young-collection roots.
#[test]
fn test_collect_young_references_rejects_invalid_root() {
    let layout = HeapOptions::default();
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let invalid = ManagedReference::new(7);

    // invalid roots should be reported directly
    let error = managed
        .collect_young_references([invalid])
        .expect_err("invalid roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidManagedReference { reference: invalid }
    );
}

/// Reject invalid non-null young references discovered while tracing.
#[test]
fn test_collect_young_references_rejects_invalid_edge() {
    let layout = HeapOptions::default();
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let invalid = ManagedReference::new(99);
    let reference_map = ReferenceMap::ReferenceOffsets {
        offsets: vec![0].into_boxed_slice(),
    };

    // encode one invalid compact managed reference into the payload
    let root = managed
        .allocate_bytes(&invalid.bits().to_le_bytes(), reference_map, None)
        .expect("managed allocation should succeed");

    let error = managed
        .collect_young_references([root])
        .expect_err("invalid traced references should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidManagedReference { reference: invalid }
    );
}

/// Leave young space intact when one minor collection fails before promotion commits.
#[test]
fn test_collect_young_references_keeps_young_space_reusable_after_failure() {
    let layout = HeapOptions::default();
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let invalid = ManagedReference::new(99);
    let reference_map = ReferenceMap::ReferenceOffsets {
        offsets: vec![0].into_boxed_slice(),
    };

    // encode one invalid edge so tracing fails before the young pass can commit
    let root = managed
        .allocate_bytes(&invalid.bits().to_le_bytes(), reference_map, None)
        .expect("managed allocation should succeed");
    let young_offset = managed.young.next_offset;

    let error = managed
        .collect_young_references([root])
        .expect_err("invalid traced references should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidManagedReference { reference: invalid }
    );
    assert!(!managed.is_collecting);
    assert_eq!(managed.gc_state().completed_cycles, 0);
    assert_eq!(managed.young.next_offset, young_offset);
    assert!(matches!(
        managed.location(root),
        Some(ManagedLocation::Young(_))
    ));
    assert_eq!(
        managed.read_bytes(root),
        Ok(invalid.bits().to_le_bytes().to_vec())
    );

    // a later valid retry should still be able to complete normally
    managed
        .set_bytes(root, 0, &ManagedReference::NULL.bits().to_le_bytes())
        .expect("managed bytes should update");
    let stats = managed
        .collect_young_references([root])
        .expect("young collection should succeed after failure");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(managed.gc_state().completed_cycles, 1);
    assert!(matches!(
        managed.location(root),
        Some(ManagedLocation::Small(_)) | Some(ManagedLocation::Large(_))
    ));
}

/// Free unreachable mature entries during full collection and record both GC cycles.
#[test]
fn test_collect_references_frees_unreachable_mature_entries_and_records_gc_state() {
    let layout = HeapOptions {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        ..HeapOptions::default()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");

    // allocate directly into mature space so the full pass must reclaim the unreachable entry
    let reachable = managed
        .allocate_bytes(&[1, 2, 3], ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let unreachable = managed
        .allocate_bytes(&[4, 5, 6], ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");

    let stats = managed
        .collect_references([reachable])
        .expect("full collection should succeed");

    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(managed.read_bytes(reachable), Ok(vec![1, 2, 3]));
    assert!(!managed.is_live(unreachable));
    assert_eq!(managed.gc_state().completed_cycles, 2);
    assert_eq!(
        managed.gc_state().last_cycle,
        Some(GcCycle {
            kind: GcKind::Full,
            stats,
        })
    );
}
