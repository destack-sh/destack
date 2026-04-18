use crate::local::managed::ManagedLocation;
use crate::tests::test_arena;
use crate::{EdgeMap, GcCycle, GcKind, HeapError, HeapOptions, ManagedReference, ManagedSpace};

/// Promote reachable young entries and clear unreachable young-space state.
#[test]
fn test_collect_young_promotes_reachable_entries() {
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::default()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");

    // allocate one reachable and one unreachable young entry
    let reachable = managed
        .allocate_bytes(&[1, 2, 3], EdgeMap::empty(), None)
        .expect("managed allocation should succeed");
    let unreachable = managed
        .allocate_bytes(&[4, 5, 6], EdgeMap::empty(), None)
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
        .collect_young([reachable])
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
fn test_collect_young_promotes_reachable_child_entries() {
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::default()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let edge_map = EdgeMap::ReferenceOffsets {
        offsets: vec![0].into_boxed_slice(),
    };

    // build one young parent that points at one young child
    let child = managed
        .allocate_bytes(&[0xC1, 0x1D], EdgeMap::empty(), None)
        .expect("managed allocation should succeed");
    let parent = managed
        .allocate_bytes(&child.bits().to_le_bytes(), edge_map, None)
        .expect("managed allocation should succeed");

    let stats = managed
        .collect_young([parent])
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(managed.read_bytes(child), Ok(vec![0xC1, 0x1D]));
    assert!(!matches!(
        managed.location(child),
        Some(ManagedLocation::Young(_))
    ));
    assert!(!matches!(
        managed.location(parent),
        Some(ManagedLocation::Young(_))
    ));
}

/// Reject invalid explicit young roots loudly.
#[test]
fn test_collect_young_rejects_invalid_root() {
    let layout = HeapOptions::default();
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let invalid = ManagedReference::new(7);

    let error = managed
        .collect_young([invalid])
        .expect_err("invalid roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidManagedReference { reference: invalid }
    );
}

/// Record the completed young GC cycle after one local young collection.
#[test]
fn test_collect_young_updates_gc_state() {
    let layout = HeapOptions::default();
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let reachable = managed
        .allocate_bytes(&[1, 2, 3], EdgeMap::empty(), None)
        .expect("managed allocation should succeed");

    let stats = managed
        .collect_young([reachable])
        .expect("young collection should succeed");

    assert_eq!(managed.gc_state().completed_cycles, 1);
    assert_eq!(
        managed.gc_state().last_cycle,
        Some(GcCycle {
            kind: GcKind::Minor,
            stats,
        })
    );
}
