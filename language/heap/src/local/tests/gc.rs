use crate::local::managed::ManagedLocation;
use crate::tests::test_arena;
use crate::{
    GcKind, GcOptions, Heap, HeapError, HeapOptions, ManagedReference, ManagedSpace,
    SharedManagedReference,
};
use destack_mir::LayoutTrace;

/// Build one local heap whose pacer triggers immediately in step-driven tests.
fn test_heap() -> Heap {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 10,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
        },
        ..HeapOptions::local()
    };

    Heap::with_limits_and_options(crate::HeapLimits::default(), options)
        .expect("pacing heap should build")
}

/// Promote reachable young entries and clear unreachable young-space state.
#[test]
fn test_collect_minor_promotes_reachable_entries() {
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");

    // allocate one reachable and one unreachable young entry
    let reachable = managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let unreachable = managed
        .allocate_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
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
        .collect_minor([reachable])
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
fn test_collect_minor_promotes_reachable_child_entries() {
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let trace = LayoutTrace::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };

    // build one young parent that points at one young child
    let child = managed
        .allocate_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let parent = managed
        .allocate_bytes(&child.bits().to_le_bytes(), trace, None)
        .expect("managed allocation should succeed");

    let stats = managed
        .collect_minor([parent])
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
fn test_collect_minor_rejects_invalid_root() {
    let layout = HeapOptions::local();
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let invalid = ManagedReference::new(7);

    let error = managed
        .collect_minor([invalid])
        .expect_err("invalid roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidManagedReference { reference: invalid }
    );
}

/// Record the completed young GC cycle after one local young collection.
#[test]
fn test_collect_minor_updates_gc_state() {
    let layout = HeapOptions::local();
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let reachable = managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");

    let stats = managed
        .collect_minor([reachable])
        .expect("young collection should succeed");

    assert_eq!(managed.gc_state().completed_cycles, 1);
    assert_eq!(managed.gc_state().last_kind, Some(GcKind::Minor));
    assert_eq!(managed.gc_state().last_stats, Some(stats));
}

/// Pinning one young reference should tenure it immediately.
#[test]
fn test_pin_promotes_young_reference() {
    // build one managed space with mature slots for promoted entries
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let reference = managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");

    // pinning should move the nursery object into stable mature storage
    assert!(matches!(
        managed.location(reference),
        Some(ManagedLocation::Young(_))
    ));

    managed.pin(reference).expect("pin should succeed");

    // the reference should stay live and stable after pinning
    assert_eq!(
        managed.pins.references().collect::<Vec<_>>(),
        vec![reference]
    );
    assert_eq!(managed.read_bytes(reference), Ok(vec![1, 2, 3]));
    assert!(matches!(
        managed.location(reference),
        Some(ManagedLocation::Small(_)) | Some(ManagedLocation::Large(_))
    ));
}

/// Pinned mature roots should keep young children alive during minor collection.
#[test]
fn test_collect_minor_traces_pinned_roots() {
    // build one managed space with traced local edges
    let layout = HeapOptions {
        managed_small_bytes: 32,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let trace = LayoutTrace::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let child = managed
        .allocate_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let parent = managed
        .allocate_bytes(&child.bits().to_le_bytes(), trace, None)
        .expect("managed allocation should succeed");

    // pinning the parent should tenure it before collection
    managed.pin(parent).expect("pin should succeed");

    let stats = managed
        .collect_minor([])
        .expect("young collection should succeed");

    // the pinned parent should keep the young child alive
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

/// Pinned mature references should stay live during full collection without explicit roots.
#[test]
fn test_collect_full_traces_pinned_roots() {
    // allocate directly into mature space so full collection is the only live root path
    let layout = HeapOptions {
        managed_young_bytes: 0,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed =
        ManagedSpace::with_options(arena, &layout).expect("explicit managed options should build");
    let reference = managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");

    // keep the mature reference pinned without passing it as one explicit root
    managed.pin(reference).expect("pin should succeed");

    let stats = managed
        .collect_full([])
        .expect("full collection should succeed");

    // the pinned reference should stay live across the full cycle
    assert_eq!(stats.freed_allocations, 0);
    assert!(managed.is_live(reference));
    assert_eq!(managed.read_bytes(reference), Ok(vec![1, 2, 3]));
}

/// Trace shared roots correctly even when local managed references are compact.
#[test]
fn test_scan_shared_roots_uses_shared_reference_width() {
    let options = HeapOptions {
        managed_reference_bytes: 4,
        ..HeapOptions::local()
    };
    let arena = test_arena(&options);
    let mut managed =
        ManagedSpace::with_options(arena, &options).expect("explicit managed options should build");
    let trace = LayoutTrace::Reference {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let shared = SharedManagedReference::new(7);
    let local = managed
        .allocate_bytes(&shared.bits().to_le_bytes(), trace, None)
        .expect("managed allocation should succeed");
    let mut roots = Vec::new();

    managed.start_shared_edge_scan();

    let work_done = managed
        .scan_shared_edge_step(&mut roots, 1)
        .expect("shared root scan should succeed");

    assert_eq!(work_done, 1);
    assert_eq!(roots, vec![shared]);
    assert!(managed.shared_edge_scan_idle());

    managed.finish_shared_edge_scan();

    assert!(managed.is_live(local));
}

/// Stay idle when no local pressure or explicit request exists.
#[test]
fn test_heap_gc_step_stays_idle_without_request() {
    let mut heap = Heap::new().expect("heap should build");

    let stats = heap.gc_step([]).expect("gc step should succeed");

    assert_eq!(stats, None);
}

/// Run one minor cycle after local managed allocation pressure.
#[test]
fn test_heap_gc_step_runs_minor_after_pressure() {
    let mut heap = test_heap();
    let root = heap
        .allocate_managed_bytes(&vec![1; 64], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");

    let stats = heap
        .gc_step([root])
        .expect("gc step should succeed")
        .expect("pressure should request one cycle");

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_managed_live(root));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
}

/// Honor one explicit full local collection request below the pacing trigger.
#[test]
fn test_heap_gc_step_honors_manual_full_request() {
    let mut heap = Heap::new().expect("heap should build");
    let root = heap
        .allocate_managed_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");

    heap.request_full_gc();

    let stats = heap
        .gc_step([root])
        .expect("gc step should succeed")
        .expect("manual request should run one cycle");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}
